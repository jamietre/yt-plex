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
