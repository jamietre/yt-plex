# Download Progress Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface yt-dlp download progress (percentage) in the UI in real-time via WebSocket.

**Architecture:** Switch the worker from `Command::output()` (buffered) to `Command::spawn()` (streaming). Read stderr line-by-line while yt-dlp runs, parse `[download] N%` lines, and broadcast progress messages over WebSocket. Add a `progress` field to `WsMessage` and display a percentage alongside the downloading status in the job table.

**Tech Stack:** Rust/Tokio (async process streaming), existing WebSocket broadcast infrastructure, SvelteKit frontend.

---

## File Map

| File | Change |
|------|--------|
| `crates/common/src/models.rs` | Add `progress: Option<f32>` to `WsMessage` |
| `crates/server/src/worker.rs` | Replace `output()` with `spawn()`, stream stderr, parse progress, broadcast |
| `crates/server/src/ws.rs` | No change needed — `broadcast()` already accepts any `WsMessage` |
| `web/src/lib/ws.ts` | Add `progress: number \| null` to `WsMessage` interface |
| `web/src/routes/+page.svelte` | Add `progress` field to job state, show percentage in status column |

---

### Task 1: Add progress field to WsMessage

**Files:**
- Modify: `crates/common/src/models.rs`

The `WsMessage` struct gains an optional progress field. The `Job` struct does **not** change — progress is transient and not persisted.

- [ ] **Step 1: Update the ws_message_serialises test to cover the new field**

In `crates/common/src/models.rs`, update the existing test:

```rust
#[test]
fn ws_message_serialises() {
    let msg = WsMessage {
        job_id: "abc".into(),
        status: JobStatus::Done,
        channel_name: Some("Chan".into()),
        title: Some("Vid".into()),
        error: None,
        progress: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"status\":\"done\""));
    // progress: None should be omitted from JSON (skip_serializing_if)
    assert!(!json.contains("progress"));

    let msg_with_progress = WsMessage {
        job_id: "abc".into(),
        status: JobStatus::Downloading,
        channel_name: None,
        title: None,
        error: None,
        progress: Some(42.5),
    };
    let json2 = serde_json::to_string(&msg_with_progress).unwrap();
    assert!(json2.contains("\"progress\":42.5"));
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test -p yt-plex-common ws_message_serialises 2>&1 | tail -10
```

Expected: compile error — `WsMessage` has no `progress` field yet.

- [ ] **Step 3: Add `progress` to WsMessage and update from_job**

In `crates/common/src/models.rs`, update `WsMessage`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub job_id: String,
    pub status: JobStatus,
    pub channel_name: Option<String>,
    pub title: Option<String>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
}

impl WsMessage {
    pub fn from_job(job: &Job) -> Self {
        Self {
            job_id: job.id.clone(),
            status: job.status.clone(),
            channel_name: job.channel_name.clone(),
            title: job.title.clone(),
            error: job.error.clone(),
            progress: None,
        }
    }
}
```

- [ ] **Step 4: Run the test to confirm it passes**

```bash
cargo test -p yt-plex-common 2>&1 | tail -10
```

Expected: all 3 common tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/common/src/models.rs
git commit -m "feat: add progress field to WsMessage"
```

---

### Task 2: Stream yt-dlp stderr and broadcast progress

**Files:**
- Modify: `crates/server/src/worker.rs`

Replace `Command::output()` with `Command::spawn()`. Read stderr line-by-line while the process runs, parsing progress. Collect stdout for JSON parsing after the process exits.

- [ ] **Step 1: Write the updated parse_progress test**

Add a new test to the existing `#[cfg(test)] mod tests` block in `crates/server/src/worker.rs`:

```rust
#[test]
fn parse_progress_extracts_percentage() {
    assert_eq!(
        parse_progress("[download]  23.5% of 45.23MiB at 2.34MiB/s ETA 00:16"),
        Some(23.5)
    );
    assert_eq!(
        parse_progress("[download] 100% of 10.00MiB at 5.00MiB/s ETA 00:00"),
        Some(100.0)
    );
    assert_eq!(parse_progress("[download] Destination: video.mp4"), None);
    assert_eq!(parse_progress("[info] some other line"), None);
    assert_eq!(parse_progress("  0.0% something"), None); // no [download] prefix
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test -p yt-plex-server parse_progress 2>&1 | tail -10
```

Expected: compile error — `parse_progress` not defined yet.

- [ ] **Step 3: Add parse_progress function**

Add this function to `crates/server/src/worker.rs` (after `parse_ytdlp_json`):

```rust
/// Extract download percentage from a yt-dlp stderr progress line.
/// Matches lines like: `[download]  23.5% of 45.23MiB at 2.34MiB/s ETA 00:16`
pub fn parse_progress(line: &str) -> Option<f32> {
    let line = line.trim();
    let rest = line.strip_prefix("[download]")?.trim();
    let pct_str = rest.split('%').next()?.trim();
    pct_str.parse::<f32>().ok()
}
```

- [ ] **Step 4: Run the test to confirm it passes**

```bash
cargo test -p yt-plex-server parse_progress 2>&1 | tail -10
```

Expected: `test worker::tests::parse_progress_extracts_percentage ... ok`

- [ ] **Step 5: Replace output() with spawn() + streaming in the tick() function**

Replace the yt-dlp invocation in `tick()`. The existing code is:

```rust
// Run yt-dlp
let output = Command::new("yt-dlp")
    .args(["--print-json", "-o", &out_template, &job.url])
    .output()
    .await
    .context("spawning yt-dlp (is it installed?)")?;

if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    db.update_job(&job.id, JobStatus::Failed, None, None, Some(&stderr))?;
    let updated = db.get_job(&job.id)?.unwrap();
    hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));
    warn!("yt-dlp failed for {}: {stderr}", job.id);
    return Ok(());
}

let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
// yt-dlp may print multiple JSON lines (playlist); take the last non-empty one
let last_line = stdout
    .lines()
    .filter(|l| !l.trim().is_empty())
    .last()
    .unwrap_or("");
```

Replace it with:

```rust
// Run yt-dlp, streaming stderr for progress updates
let mut child = Command::new("yt-dlp")
    .args(["--newline", "--print-json", "-o", &out_template, &job.url])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .context("spawning yt-dlp (is it installed?)")?;

let stderr_pipe = child.stderr.take().expect("stderr piped");
let mut stderr_lines = tokio::io::BufReader::new(stderr_pipe).lines();
let mut stderr_buf = String::new();

// Stream stderr: parse progress lines and broadcast, accumulate the rest
while let Ok(Some(line)) = stderr_lines.next_line().await {
    if let Some(pct) = parse_progress(&line) {
        hub.broadcast(&yt_plex_common::models::WsMessage {
            job_id: job.id.clone(),
            status: JobStatus::Downloading,
            channel_name: None,
            title: None,
            error: None,
            progress: Some(pct),
        });
    }
    stderr_buf.push_str(&line);
    stderr_buf.push('\n');
}

let output = child.wait_with_output().await.context("waiting for yt-dlp")?;

if !output.status.success() {
    db.update_job(&job.id, JobStatus::Failed, None, None, Some(&stderr_buf))?;
    let updated = db.get_job(&job.id)?.unwrap();
    hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));
    warn!("yt-dlp failed for {}: {stderr_buf}", job.id);
    return Ok(());
}

let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
// yt-dlp may print multiple JSON lines (playlist); take the last non-empty one
let last_line = stdout
    .lines()
    .filter(|l| !l.trim().is_empty())
    .last()
    .unwrap_or("");
```

Also add this import at the top of the file alongside the existing imports:

```rust
use tokio::io::AsyncBufReadExt;
```

- [ ] **Step 6: Build and run all tests**

```bash
cargo build -p yt-plex-server 2>&1 | grep "^error" | head -20
cargo test -p yt-plex-server 2>&1 | tail -15
```

Expected: clean build, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/server/src/worker.rs
git commit -m "feat: stream yt-dlp progress and broadcast via WebSocket"
```

---

### Task 3: Update frontend to show progress

**Files:**
- Modify: `web/src/lib/ws.ts`
- Modify: `web/src/routes/+page.svelte`

- [ ] **Step 1: Add progress to WsMessage interface in ws.ts**

In `web/src/lib/ws.ts`, update `WsMessage`:

```typescript
export interface WsMessage {
    job_id: string;
    status: Job['status'];
    channel_name: string | null;
    title: string | null;
    error: string | null;
    progress?: number | null;
}
```

- [ ] **Step 2: Add progress to the Job type in api.ts**

In `web/src/lib/api.ts`, the `Job` interface needs a `progress` field so the jobs array can hold transient progress state:

```typescript
export interface Job {
    id: string;
    url: string;
    status: 'queued' | 'downloading' | 'copying' | 'done' | 'failed';
    channel_name: string | null;
    title: string | null;
    error: string | null;
    created_at: string;
    updated_at: string;
    progress?: number | null;  // transient, not from server REST response
}
```

- [ ] **Step 3: Apply progress in the WebSocket effect in +page.svelte**

In `web/src/routes/+page.svelte`, update the `$effect` that applies WS updates to also carry progress through:

```typescript
$effect(() => {
    const msg = $ws;
    if (msg) {
        jobs = jobs.map(j =>
            j.id === msg.job_id
                ? {
                    ...j,
                    status: msg.status,
                    channel_name: msg.channel_name ?? j.channel_name,
                    title: msg.title ?? j.title,
                    error: msg.error,
                    progress: msg.progress ?? (msg.status !== 'downloading' ? null : j.progress),
                  }
                : j
        );
    }
});
```

This clears progress when the job leaves the `downloading` state.

- [ ] **Step 4: Show progress in the status column**

In `web/src/routes/+page.svelte`, update the status cell in the table:

```svelte
<td style="color:{statusColour[job.status]}">
    {job.status}
    {#if job.status === 'downloading' && job.progress != null}
        <span class="progress">{job.progress.toFixed(0)}%</span>
    {/if}
</td>
```

Add to the `<style>` block:

```css
.progress { font-size: 0.85em; opacity: 0.8; margin-left: 0.3em; }
```

- [ ] **Step 5: Build the frontend**

```bash
cd web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 6: Commit**

```bash
git add web/src/lib/ws.ts web/src/lib/api.ts web/src/routes/+page.svelte
git commit -m "feat: show download progress percentage in job table"
```

---

### Task 4: Final verification

- [ ] **Step 1: Full test suite**

```bash
cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all tests pass.

- [ ] **Step 2: Release build**

```bash
cargo build --release -p yt-plex-server 2>&1 | grep "^error" | head -5
```

Expected: clean.

- [ ] **Step 3: Push**

```bash
git push
```
