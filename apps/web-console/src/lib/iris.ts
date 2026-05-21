import { IrisClient } from "@iris/sdk-ts";

/**
 * Backend base URL. Set via NEXT_PUBLIC_IRIS_BASE_URL, defaults to localhost.
 * For tailscale deployments override at build/runtime.
 */
export const IRIS_BASE_URL =
  process.env.NEXT_PUBLIC_IRIS_BASE_URL ?? "http://127.0.0.1:8787";

export const irisClient = new IrisClient({ baseUrl: IRIS_BASE_URL });
