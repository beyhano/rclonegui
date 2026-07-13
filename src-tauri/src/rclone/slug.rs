/// Generate a programmatic slug from a user-friendly name.
///
/// Rules:
/// - Lowercase
/// - Turkish chars → ASCII (ş→s, ı→i, ü→u, ö→o, ç→c, ğ→g)
/// - Spaces → hyphens
/// - Remove non-alphanumeric chars (except hyphens)
/// - Collapse multiple hyphens into one
/// - Trim leading/trailing hyphens
/// - If result is empty, return "task"

pub fn generate_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| match c {
            'ş' | 'Ş' => 's',
            'ı' | 'I' => 'i',
            'İ' => 'i',
            'ü' | 'Ü' => 'u',
            'ö' | 'Ö' => 'o',
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            ' ' | '_' => '-',
            c if c.is_alphanumeric() || c == '-' => c,
            _ => '-',
        })
        .collect::<String>()
        .to_lowercase();

    // Collapse multiple hyphens
    let cleaned: String = slug
        .chars()
        .fold(String::new(), |mut acc, c| {
            if c == '-' && acc.ends_with('-') {
                // skip duplicate
            } else {
                acc.push(c);
            }
            acc
        })
        .trim_matches('-')
        .to_string();

    if cleaned.is_empty() { "task".to_string() } else { cleaned }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_slug() { assert_eq!(generate_slug("Daily Backup"), "daily-backup"); }
    #[test]
    fn test_turkish_chars() {
        assert_eq!(generate_slug("Yedekleme İşi 2"), "yedekleme-isi-2");
        assert_eq!(generate_slug("Şemsiye örneği"), "semsiye-ornegi");
        assert_eq!(generate_slug("Çöp Ğüş"), "cop-gus");
    }
    #[test]
    fn test_special_chars() { assert_eq!(generate_slug("Hello!!! World??"), "hello-world"); }
    #[test]
    fn test_multiple_hyphens() { assert_eq!(generate_slug("a   b---c"), "a-b-c"); }
    #[test]
    fn test_trim_hyphens() { assert_eq!(generate_slug("--hello--"), "hello"); }
    #[test]
    fn test_empty_becomes_task() { assert_eq!(generate_slug("!!!   ???"), "task"); }
}
