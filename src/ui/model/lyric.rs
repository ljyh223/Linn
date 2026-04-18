#[derive(Debug)]
pub struct LyricChar {
    pub ch: String,
    pub start: u32,
    pub duration: u32,
}

#[derive(Debug)]
pub struct LyricLine {
    pub start: u32,
    pub duration: u32,
    pub chars: Vec<LyricChar>,
}

#[derive(Debug)]
pub struct LrcLine {
    pub time_ms: u32,
    pub text: String,
}