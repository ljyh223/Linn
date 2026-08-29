use chrono::Timelike;
use image::GenericImageView;

pub fn format_duration(ms: u64) -> String {
    let s = ms / 1000;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn format_number(num: u64) -> String {
    match num {
        n if n >= 100_000_000 => format!("{:.1}亿", n as f64 / 100_000_000.0),
        n if n >= 10_000 => format!("{:.1}万", n as f64 / 10_000.0),
        _ => num.to_string(),
    }
}

/// 为网易云图片地址附加尺寸参数，兼容接口已返回查询参数的 URL。
///
/// 部分新版接口返回 `...?imageView=1&type=webp&thumbnail=...`；继续拼接
/// `?param=` 会生成两个问号并导致图片请求失败。
pub fn image_url(url: impl AsRef<str>, size: &str) -> String {
    let url = url.as_ref();
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}param={size}")
}

#[cfg(test)]
mod tests {
    use super::image_url;

    #[test]
    fn image_size_parameter_uses_the_existing_query_separator() {
        assert_eq!(
            image_url("http://p3.music.126.net/cover.jpg", "300y300"),
            "http://p3.music.126.net/cover.jpg?param=300y300"
        );
        assert_eq!(
            image_url(
                "http://p3.music.126.net/cover.jpg?imageView=1&type=webp&thumbnail=400y400",
                "300y300"
            ),
            "http://p3.music.126.net/cover.jpg?imageView=1&type=webp&thumbnail=400y400&param=300y300"
        );
    }
}

pub fn extract_dominant_color(image_bytes: &[u8]) -> String {
    let img = match image::load_from_memory(image_bytes) {
        Ok(img) => img,
        Err(_) => return "#333333".into(),
    };

    let small = img.resize_exact(32, 32, image::imageops::FilterType::Triangle);
    let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
    let count = (small.width() * small.height()) as u64;

    for pixel in small.pixels() {
        let [pr, pg, pb, _] = pixel.2.0;
        r += pr as u64;
        g += pg as u64;
        b += pb as u64;
    }

    r /= count;
    g /= count;
    b /= count;

    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

pub fn time_greeting() -> &'static str {
    let hour = chrono::Local::now().hour();
    match hour {
        6..=10 => "早上好",
        11..=13 => "中午好",
        14..=17 => "下午好",
        18..=22 => "晚上好",
        _ => "夜深了",
    }
}
