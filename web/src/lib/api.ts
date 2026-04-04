export interface Job {
    id: string;
    url: string;
    status: 'queued' | 'downloading' | 'copying' | 'done' | 'failed';
    channel_name: string | null;
    title: string | null;
    error: string | null;
    created_at: string;
    updated_at: string;
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

export async function login(password: string): Promise<void> {
    const res = await fetch('/api/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password }),
    });
    if (!res.ok) throw new Error('Invalid password');
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
