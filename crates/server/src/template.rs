/// Render a path template with the given variables.
/// `date` must be in `YYYY-MM-DD` format.
/// Sanitises channel and title to remove `/` and `\` to avoid path traversal.
pub fn render(
    template: &str,
    channel: &str,
    channel_id: &str,
    date: &str,
    title: &str,
    ext: &str,
    id: &str,
) -> String {
    let channel = sanitise(channel);
    let title = sanitise(title);
    // Extract year/month/day from YYYY-MM-DD; fall back to the full date string on malformed input.
    let (yyyy, mm, dd) = if date.len() == 10 && date.as_bytes()[4] == b'-' && date.as_bytes()[7] == b'-' {
        (&date[..4], &date[5..7], &date[8..10])
    } else {
        (date, "", "")
    };
    template
        .replace("{channel}", &channel)
        .replace("{channel_id}", channel_id)
        .replace("{date}", date)
        .replace("{yyyy}", yyyy)
        .replace("{mm}", mm)
        .replace("{dd}", dd)
        .replace("{title}", &title)
        .replace("{ext}", ext)
        .replace("{id}", id)
}

fn sanitise(s: &str) -> String {
    s.replace(['/', '\\'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_variables() {
        let result = render(
            "{channel}/{date} - {title} [{id}].{ext}",
            "MyChan",
            "UCxxx",
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
            "UCxxx",
            "2026-04-04",
            "Vid",
            "mp4",
            "abc123",
        );
        assert_eq!(result, "Chan/2026-04-04 - Vid.mp4");
    }

    #[test]
    fn sanitises_path_separators_in_title() {
        let result = render("{title}.{ext}", "Chan", "UCxxx", "2026-04-04", "foo/bar", "mp4", "id1");
        assert_eq!(result, "foo_bar.mp4");
    }

    #[test]
    fn renders_yyyy_mm_dd_variables() {
        let result = render(
            "{channel}/Season {yyyy}/{yyyy}-{mm}-{dd} - {title} [{id}].{ext}",
            "MyChan",
            "UCxxx",
            "2026-04-04",
            "My Video",
            "mp4",
            "dQw4w9WgXcQ",
        );
        assert_eq!(result, "MyChan/Season 2026/2026-04-04 - My Video [dQw4w9WgXcQ].mp4");
    }

    #[test]
    fn renders_channel_id_variable() {
        let result = render(
            "{channel} [{channel_id}]/Season {yyyy}/{title} [{id}].{ext}",
            "Veritasium",
            "UCHnyfMqiRRz1Pbc3OkCkEug",
            "2026-03-28",
            "My Video",
            "mp4",
            "lW4FetrdEK4",
        );
        assert_eq!(
            result,
            "Veritasium [UCHnyfMqiRRz1Pbc3OkCkEug]/Season 2026/My Video [lW4FetrdEK4].mp4"
        );
    }
}
