/// Render a path template with the given variables.
/// Sanitises channel and title to remove `/` and `\` to avoid path traversal.
pub fn render(template: &str, channel: &str, date: &str, title: &str, ext: &str) -> String {
    let channel = sanitise(channel);
    let title = sanitise(title);
    template
        .replace("{channel}", &channel)
        .replace("{date}", date)
        .replace("{title}", &title)
        .replace("{ext}", ext)
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
            "{channel}/{date} - {title}.{ext}",
            "MyChan",
            "2026-04-04",
            "My Video",
            "mp4",
        );
        assert_eq!(result, "MyChan/2026-04-04 - My Video.mp4");
    }

    #[test]
    fn sanitises_path_separators_in_title() {
        let result = render("{title}.{ext}", "Chan", "2026-04-04", "foo/bar", "mp4");
        assert_eq!(result, "foo_bar.mp4");
    }
}
