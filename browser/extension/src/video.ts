/** Complete-video operations, independent of the live tab audio capture. */
export type VideoStage =
  | "queued"
  | "downloading"
  | "converting"
  | "transcribing"
  | "cancelling"
  | "completed"
  | "cancelled"
  | "failed";

export interface VideoJob {
  id: string;
  stage: VideoStage;
  url: string;
  result: {
    directory: string;
    video: string;
    srt: string;
    vtt: string;
    document: string;
  } | null;
  error: string | null;
}

/** Source and timed text consumed by a future summarization feature. */
export interface VideoDocument {
  schema_version: 1;
  source: { url: string; id: string; title: string };
  transcript: {
    schema_version: 1;
    source: string;
    language: string | null;
    segments: { id: number; start_ms: number; end_ms: number; text: string }[];
  };
}

export type VideoAction =
  { kind: "submit"; url: string } | { kind: "status" | "cancel" | "document"; id: string };

const endpoint = "http://127.0.0.1:9743/v1/video/jobs";

async function request<T>(token: string, path: string, method: string, body?: object): Promise<T> {
  if (!/^[a-fA-F0-9]{64}$/.test(token)) throw new Error("配对令牌需要 64 位十六进制字符");
  const response = await fetch(endpoint + path, {
    method,
    headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
    ...(body ? { body: JSON.stringify(body) } : {}),
    signal: AbortSignal.timeout(10_000),
    credentials: "omit",
    cache: "no-store",
  });
  if (!response.ok) {
    const detail = (await response.json().catch(() => null)) as { message?: string } | null;
    throw new Error(detail?.message ?? `视频任务请求失败（${response.status}）`);
  }
  return (await response.json()) as T;
}

function jobPath(id: string): string {
  if (!/^[a-f0-9]{32}$/.test(id)) throw new Error("无效的视频任务 ID");
  return `/${id}`;
}

export function submitVideo(token: string, url: string): Promise<VideoJob> {
  const parsed = new URL(url);
  if (!["http:", "https:"].includes(parsed.protocol))
    throw new Error("视频链接需要使用 HTTP 或 HTTPS");
  return request(token, "", "POST", { url });
}

export function getVideoJob(token: string, id: string): Promise<VideoJob> {
  return request(token, jobPath(id), "GET");
}

export function cancelVideoJob(token: string, id: string): Promise<VideoJob> {
  return request(token, jobPath(id), "DELETE");
}

export function getVideoDocument(token: string, id: string): Promise<VideoDocument> {
  return request(token, `${jobPath(id)}/document`, "GET");
}

export async function handleVideoAction(
  token: string,
  action: VideoAction,
): Promise<VideoJob | VideoDocument> {
  switch (action.kind) {
    case "submit":
      return submitVideo(token, action.url);
    case "status":
      return getVideoJob(token, action.id);
    case "cancel":
      return cancelVideoJob(token, action.id);
    case "document":
      return getVideoDocument(token, action.id);
  }
  throw new Error("无效的视频任务操作");
}
