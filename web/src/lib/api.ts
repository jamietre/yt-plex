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


export async function logout(): Promise<void> {
    await fetch('/api/logout', { method: 'POST' });
}

export type AuthFlow = 'authorization_code' | 'device';

export async function getAuthFlow(): Promise<AuthFlow> {
    const res = await fetch('/api/auth/flow');
    if (!res.ok) return 'authorization_code';
    const data = await res.json();
    return data.flow === 'device' ? 'device' : 'authorization_code';
}

export interface DeviceLoginResponse {
    poll_token: string;
    user_code: string;
    verification_url: string;
    interval: number;
    expires_in: number;
}

export async function startDeviceLogin(): Promise<DeviceLoginResponse> {
    const res = await fetch('/api/auth/device', { method: 'POST' });
    if (!res.ok) throw new Error(`startDeviceLogin failed: ${res.status}`);
    return res.json();
}

export type PollStatus = 'pending' | 'done' | 'expired';

export async function pollDeviceAuth(pollToken: string): Promise<PollStatus> {
    const res = await fetch('/api/auth/poll', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ poll_token: pollToken }),
    });
    if (!res.ok) return 'expired';
    const data = await res.json();
    return data.status;
}

export interface Settings {
    plex: { url: string; token: string; library_section_id: string };
    output: { base_path: string; path_template: string; thumbnail_cache_dir: string };
    download: { extra_args: string[] };
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

export interface PlexLibrary {
    id: string;
    title: string;
    lib_type: string;
}

export async function listPlexLibraries(): Promise<PlexLibrary[]> {
    const res = await fetch('/api/plex/libraries');
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `listPlexLibraries failed: ${res.status}`);
    }
    return res.json();
}


export type VideoStatus = 'new' | 'in_progress' | 'downloaded' | 'ignored';

export interface Channel {
    id: string;
    youtube_channel_url: string;
    youtube_channel_id: string | null;
    name: string;
    last_synced_at: string | null;
    path_prefix: string | null;
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

export async function listAllChannels(): Promise<Channel[]> {
    const res = await fetch('/api/channels?all=true');
    if (!res.ok) throw new Error(`listAllChannels failed: ${res.status}`);
    return res.json();
}

export async function addChannel(url: string, name: string, pathPrefix?: string): Promise<Channel> {
    const res = await fetch('/api/channels', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url, name, path_prefix: pathPrefix || null }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `addChannel failed: ${res.status}`);
    }
    return res.json();
}

export async function updateChannel(id: string, name: string, url: string, pathPrefix?: string): Promise<Channel> {
    const res = await fetch(`/api/channels/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, url, path_prefix: pathPrefix || null }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `updateChannel failed: ${res.status}`);
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

export async function regenChannelMetadata(id: string): Promise<{ queued: number }> {
    const res = await fetch(`/api/channels/${id}/regen-metadata`, { method: 'POST' });
    if (!res.ok) throw new Error(`regenMetadata failed: ${res.status}`);
    return res.json();
}

export async function rescanFilesystem(): Promise<void> {
    const res = await fetch('/api/rescan', { method: 'POST' });
    if (!res.ok) throw new Error(`rescan failed: ${res.status}`);
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

export interface Profile {
    id: number;
    name: string;
    linked_email: string | null;
    is_admin_profile: boolean;
    created_at: string;
}

export async function listProfiles(): Promise<Profile[]> {
    const res = await fetch('/api/profiles');
    if (!res.ok) throw new Error(`listProfiles failed: ${res.status}`);
    return res.json();
}

export async function createProfile(name: string): Promise<Profile> {
    const res = await fetch('/api/profiles', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `createProfile failed: ${res.status}`);
    }
    return res.json();
}

export async function deleteProfile(id: number): Promise<void> {
    const res = await fetch(`/api/profiles/${id}`, { method: 'DELETE' });
    if (!res.ok) throw new Error(`deleteProfile failed: ${res.status}`);
}

/** Returns the current profile, or null if none is selected. */
export async function getProfileSession(): Promise<Profile | null> {
    const res = await fetch('/api/profile-session');
    if (res.status === 204) return null;
    if (!res.ok) return null;
    return res.json();
}

export async function setProfileSession(profileId: number): Promise<void> {
    const res = await fetch('/api/profile-session', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ profile_id: profileId }),
    });
    if (!res.ok) throw new Error(`setProfileSession failed: ${res.status}`);
}

export async function clearProfileSession(): Promise<void> {
    await fetch('/api/profile-session', { method: 'DELETE' });
}

/** Returns the admin profile linked to the current session, or null if not logged in. */
export async function getAdminProfile(): Promise<Profile | null> {
    const res = await fetch('/api/auth/admin-profile');
    if (!res.ok) return null;
    return res.json();
}

export async function listProfileChannelIds(profileId: number): Promise<string[]> {
    const res = await fetch(`/api/profiles/${profileId}/channels`);
    if (!res.ok) throw new Error(`listProfileChannelIds failed: ${res.status}`);
    return res.json();
}

export async function subscribeChannel(profileId: number, channelId: string): Promise<void> {
    const res = await fetch(`/api/profiles/${profileId}/channels/${channelId}`, { method: 'PUT' });
    if (!res.ok) throw new Error(`subscribeChannel failed: ${res.status}`);
}

export async function unsubscribeChannel(profileId: number, channelId: string): Promise<void> {
    const res = await fetch(`/api/profiles/${profileId}/channels/${channelId}`, { method: 'DELETE' });
    if (!res.ok) throw new Error(`unsubscribeChannel failed: ${res.status}`);
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
