/// Render a path template with the given variables.
/// Sanitises channel and title to remove `/` and `\` to avoid path traversal.
pub fn render(template: &str, channel: &str, date: &str, title: &str, ext: &str, id: &str) -> String {
    let channel = sanitise(channel);
    let title = sanitise(title);
    template
        .replace("{channel}", &channel)
        .replace("{date}", date)
        .replace("{title}", &title)
        .replace("{ext}", ext)
        .replace("{id}", id)
}

fn sanitise(s: &str) -> String {
    s.replace('/', "_").replace('\\', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_variables() {
        let result = render(
            "{channel}/{date} - {title} [{id}].{ext}",
            "MyChan",
            "2026-04-04",
            "My Video",
            "mp4",
            "dQw4w9WgXcQ",
        );
        assert_eq!(result, "MyChan/2026-04-04 - My Video [dQw4w9WgXcQ].mp4");
    }

    #[test]
    fn renders_without_id_placeholder() {
        let result = render(
            "{channel}/{date} - {title}.{ext}",
            "Chan",
            "2026-04-04",
            "Vid",
            "mp4",
            "abc123",
        );
        assert_eq!(result, "Chan/2026-04-04 - Vid.mp4");
    }

    #[test]
    fn sanitises_path_separators_in_title() {
        let result = render("{title}.{ext}", "Chan", "2026-04-04", "foo/bar", "mp4", "id1");
        assert_eq!(result, "foo_bar.mp4");
    }
}
