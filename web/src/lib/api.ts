export interface Job {
    id: string;
    url: string;
    status: 'queued' | 'downloading' | 'copying' | 'done' | 'failed';
    channel_name: string | null;
    title: string | null;
    error: string | null;
    created_at: string;
    updated_at: string;
    progress?: number | null;
}

export async function listJobs(): Promise<Job[]> {
    const res = await fetch('/api/jobs');
    if (!res.ok) throw new Error(`listJobs failed: ${res.status}`);
    return res.json();
}

export async function submitJob(url: string): Promise<Job> {
    const res = await fetch('/api/jobs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `submit failed: ${res.status}`);
    }
    return res.json();
}

export interface DeviceLoginResponse {
  poll_token: string;
  user_code: string;
  verification_url: string;
  expires_in: number;
  interval: number;
}

export interface PollResponse {
  status: 'pending' | 'done' | 'denied' | 'expired' | 'error';
  interval?: number;
  message?: string;
}

export async function startDeviceLogin(): Promise<DeviceLoginResponse> {
  const res = await fetch('/api/auth/login');
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}

export async function pollDeviceAuth(token: string): Promise<PollResponse> {
  const res = await fetch(`/api/auth/poll?token=${encodeURIComponent(token)}`);
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}

export async function logout(): Promise<void> {
    await fetch('/api/logout', { method: 'POST' });
}

export interface Settings {
    plex: { url: string; token: string; library_section_id: string };
    output: { base_path: string; path_template: string };
}

export async function getSettings(): Promise<Settings> {
    const res = await fetch('/api/settings');
    if (!res.ok) throw new Error(`getSettings failed: ${res.status}`);
    return res.json();
}

export async function updateSettings(s: Settings): Promise<void> {
    const res = await fetch('/api/settings', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(s),
    });
    if (!res.ok) throw new Error(`updateSettings failed: ${res.status}`);
}

export type VideoStatus = 'new' | 'in_progress' | 'downloaded' | 'ignored';

export interface Channel {
    id: string;
    youtube_channel_url: string;
    name: string;
    last_synced_at: string | null;
}

export interface Video {
    youtube_id: string;
    channel_id: string;
    title: string;
    published_at: string | null;
    downloaded_at: string | null;
    last_seen_at: string;
    ignored_at: string | null;
    status: VideoStatus;
    description: string | null;
    file_path: string | null;
}

export interface VideoPage {
    videos: Video[];
    has_more: boolean;
}

export async function listChannels(): Promise<Channel[]> {
    const res = await fetch('/api/channels');
    if (!res.ok) throw new Error(`listChannels failed: ${res.status}`);
    return res.json();
}

export async function addChannel(url: string, name: string): Promise<Channel> {
    const res = await fetch('/api/channels', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url, name }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `addChannel failed: ${res.status}`);
    }
    return res.json();
}

export async function deleteChannel(id: string): Promise<void> {
    const res = await fetch(`/api/channels/${id}`, { method: 'DELETE' });
    if (!res.ok) throw new Error(`deleteChannel failed: ${res.status}`);
}

export async function syncChannel(id: string): Promise<void> {
    const res = await fetch(`/api/channels/${id}/sync`, { method: 'POST' });
    if (!res.ok) throw new Error(`syncChannel failed: ${res.status}`);
}

export async function listVideos(
    channelId: string,
    filter: 'new' | 'downloaded' | 'all' = 'new',
    showIgnored = false,
    search = '',
    limit = 48,
    offset = 0,
): Promise<VideoPage> {
    const params = new URLSearchParams({ filter, limit: String(limit), offset: String(offset) });
    if (showIgnored) params.set('show_ignored', 'true');
    if (search) params.set('q', search);
    const res = await fetch(`/api/channels/${channelId}/videos?${params}`);
    if (!res.ok) throw new Error(`listVideos failed: ${res.status}`);
    return res.json();
}

export async function ignoreVideo(youtubeId: string): Promise<void> {
    const res = await fetch(`/api/videos/${youtubeId}/ignore`, { method: 'POST' });
    if (!res.ok) throw new Error(`ignoreVideo failed: ${res.status}`);
}

export async function unignoreVideo(youtubeId: string): Promise<void> {
    const res = await fetch(`/api/videos/${youtubeId}/ignore`, { method: 'DELETE' });
    if (!res.ok) throw new Error(`unignoreVideo failed: ${res.status}`);
}

export async function getVideo(youtubeId: string): Promise<Video> {
    const res = await fetch(`/api/videos/${youtubeId}`);
    if (!res.ok) throw new Error(`getVideo failed: ${res.status}`);
    return res.json();
}

export async function submitJobByYoutubeId(youtubeId: string): Promise<Job> {
    const res = await fetch('/api/jobs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ youtube_id: youtubeId }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `submitJob failed: ${res.status}`);
    }
    return res.json();
}
