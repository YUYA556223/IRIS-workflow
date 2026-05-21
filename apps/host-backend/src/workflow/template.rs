//! `{{ <path> }}` 形式のプレースホルダ展開。
//!
//! - パスは `.` 区切り (例: `summarize.text`、`trigger.user.name`)
//! - 先頭セグメントは `trigger` (起動時データ) もしくは別ノードの id
//! - 値は `serde_json::Value` で取り出し、文字列以外は `serde_json::to_string` で stringify
//! - 文字列だけでなく、配列・オブジェクトのフィールドも再帰的にレンダする

use std::collections::HashMap;

use serde_json::Value;

/// テンプレート解決の文脈。
pub struct TemplateContext<'a> {
    pub trigger: &'a Value,
    pub outputs: &'a HashMap<String, Value>,
}

/// `Value` 構造を再帰的にスキャンし、文字列フィールド内の `{{ ... }}` を展開する。
pub fn render_value(v: &Value, ctx: &TemplateContext) -> anyhow::Result<Value> {
    match v {
        Value::String(s) => Ok(Value::String(render_string(s, ctx)?)),
        Value::Array(arr) => {
            let resolved: Result<Vec<_>, _> =
                arr.iter().map(|item| render_value(item, ctx)).collect();
            Ok(Value::Array(resolved?))
        }
        Value::Object(obj) => {
            let mut out = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj {
                out.insert(k.clone(), render_value(v, ctx)?);
            }
            Ok(Value::Object(out))
        }
        _ => Ok(v.clone()),
    }
}

pub fn render_string(s: &str, ctx: &TemplateContext) -> anyhow::Result<String> {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // `{{` 検出
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // 終端 `}}` を探す
            let start = i + 2;
            let mut end = None;
            let mut j = start;
            while j + 1 < bytes.len() {
                if bytes[j] == b'}' && bytes[j + 1] == b'}' {
                    end = Some(j);
                    break;
                }
                j += 1;
            }
            let Some(end) = end else {
                anyhow::bail!("unclosed '{{{{' in template");
            };
            let path = std::str::from_utf8(&bytes[start..end])?.trim();
            let value = resolve_path(path, ctx)?;
            out.push_str(&stringify(&value)?);
            i = end + 2; // skip `}}`
            continue;
        }
        // ascii safe: bytes[i] が UTF-8 連続バイトでも push し直すには
        // char 単位の方が確実。素直に char で進める。
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    Ok(out)
}

fn resolve_path(path: &str, ctx: &TemplateContext) -> anyhow::Result<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        anyhow::bail!("empty template path");
    }
    let first = parts[0];

    // Secrets resolver (P3.4): `{{ secrets.NAME }}` を環境変数 `IRIS_SECRET_NAME`
    // に解決する。秘密情報を YAML に書かずに済むようにするための仕組み。
    if first == "secrets" {
        if parts.len() != 2 {
            anyhow::bail!("secrets path must be 'secrets.NAME', got '{}'", path);
        }
        let name = parts[1];
        if name.is_empty() {
            anyhow::bail!("secret name must not be empty");
        }
        let env_name = format!("IRIS_SECRET_{}", name);
        let value = std::env::var(&env_name).unwrap_or_default();
        return Ok(Value::String(value));
    }

    let mut current: &Value = if first == "trigger" {
        ctx.trigger
    } else {
        ctx.outputs
            .get(first)
            .ok_or_else(|| anyhow::anyhow!("unknown node id '{}' in template", first))?
    };
    for part in &parts[1..] {
        current = match current {
            Value::Object(m) => m
                .get(*part)
                .ok_or_else(|| anyhow::anyhow!("field '{}' missing in '{}'", part, path))?,
            Value::Array(arr) => {
                let idx: usize = part
                    .parse()
                    .map_err(|_| anyhow::anyhow!("array index '{}' is not a number", part))?;
                arr.get(idx)
                    .ok_or_else(|| anyhow::anyhow!("array index {} out of range", idx))?
            }
            _ => anyhow::bail!("cannot index into scalar at '{}'", path),
        };
    }
    Ok(current.clone())
}

fn stringify(v: &Value) -> anyhow::Result<String> {
    Ok(match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => serde_json::to_string(v)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_plain_string() {
        let outputs = HashMap::new();
        let trigger = Value::Null;
        let ctx = TemplateContext {
            trigger: &trigger,
            outputs: &outputs,
        };
        assert_eq!(render_string("no placeholders", &ctx).unwrap(), "no placeholders");
    }

    #[test]
    fn renders_node_field() {
        let mut outputs = HashMap::new();
        outputs.insert("greet".to_owned(), serde_json::json!({"text": "hello"}));
        let trigger = Value::Null;
        let ctx = TemplateContext {
            trigger: &trigger,
            outputs: &outputs,
        };
        assert_eq!(render_string("X={{greet.text}}", &ctx).unwrap(), "X=hello");
    }

    #[test]
    fn renders_trigger() {
        let outputs = HashMap::new();
        let trigger = serde_json::json!({"user": "yuya"});
        let ctx = TemplateContext {
            trigger: &trigger,
            outputs: &outputs,
        };
        assert_eq!(
            render_string("hi {{ trigger.user }}", &ctx).unwrap(),
            "hi yuya"
        );
    }

    #[test]
    fn renders_secret_from_env() {
        // Set + unset must serialize in this test (otherwise other tests could
        // see it). We use a unique env var name.
        unsafe { std::env::set_var("IRIS_SECRET_TEST_TOKEN", "shhh") };
        let outputs = HashMap::new();
        let trigger = Value::Null;
        let ctx = TemplateContext {
            trigger: &trigger,
            outputs: &outputs,
        };
        let out = render_string("token=[{{ secrets.TEST_TOKEN }}]", &ctx).unwrap();
        assert_eq!(out, "token=[shhh]");
        unsafe { std::env::remove_var("IRIS_SECRET_TEST_TOKEN") };
    }
}
