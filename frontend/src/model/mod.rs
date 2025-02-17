use shrinkwraprs::Shrinkwrap;
use subsonic_types::response::Child as Song;

// TODO shrinkwrap is convenient but we should probably store the base url inline instead of having to pass
// it around :m
#[derive(Shrinkwrap, Debug, Clone, PartialEq)]
pub struct SongInfo(pub Song);

pub(crate) mod globals;
impl SongInfo {
    pub fn duration_humanized(&self) -> String {
        let total_secs = self.duration.unwrap_or_default().to_duration().as_secs();
        if total_secs == 0 {
            return "-".to_string();
        }
        let mins = total_secs / 60;
        let hours = mins / 60;
        let secs = total_secs % 60;
        let res = match total_secs {
            0..60 => format!("0:{secs:02}"),
            60..3600 => format!("{mins:02}:{secs:02}"),
            _ => format!("{hours}:{mins:02}:{secs:02}"),
        };
        res
    }

    pub fn title_with_optional_artist(&self) -> String {
        self.artist
            .as_ref()
            .map(|a| a.to_owned() + " - ")
            .unwrap_or_default()
            + &self.title
    }

    pub fn cover_art_url(&self, base_url: &str) -> Option<String> {
        self.cover_art
            .as_ref()
            .map(|id| format!("{}/rest/getCoverArt.view?id={}", base_url, id))
    }
    pub fn stream_url(&self, base_url: &str) -> String {
        format!("{}/rest/stream.view?id={}", base_url, self.id)
    }
}
