use rosu_pp::model::hit_object::HitSoundType;

pub(crate) trait Rim {
    fn rim(self) -> bool;
}

impl Rim for HitSoundType {
    fn rim(self) -> bool {
        self.has_flag(Self::CLAP | Self::WHISTLE)
    }
}
