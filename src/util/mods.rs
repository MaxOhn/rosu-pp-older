use rosu_mods::{GameMod, GameModIntermode};
use rosu_pp::{model::mods::reexports::GameModsLegacy, GameMods};

pub trait Mods {
    fn nf(&self) -> bool;
    fn ez(&self) -> bool;
    fn td(&self) -> bool;
    fn hd(&self) -> bool;
    fn hr(&self) -> bool;
    fn dt(&self) -> bool;
    fn rx(&self) -> bool;
    fn ht(&self) -> bool;
    fn fl(&self) -> bool;
    fn so(&self) -> bool;
    fn bl(&self) -> bool;
    fn tc(&self) -> bool;

    fn clock_rate(&self) -> f64 {
        if self.dt() {
            1.5
        } else if self.ht() {
            0.75
        } else {
            1.0
        }
    }

    fn no_slider_head_acc(&self, lazer: bool) -> bool;

    fn reflection(&self) -> Reflection;
}

macro_rules! impl_mods_fn {
    ( $fn_name:ident, false ) => {
        fn $fn_name(&self) -> bool {
            false
        }
    };

    ( $fn_name:ident, $bits:expr ) => {
        fn $fn_name(&self) -> bool {
            *self & ($bits) != 0
        }
    };
}

impl Mods for u32 {
    impl_mods_fn!(nf, 1 << 0);
    impl_mods_fn!(ez, 1 << 1);
    impl_mods_fn!(td, 1 << 2);
    impl_mods_fn!(hd, 1 << 3);
    impl_mods_fn!(hr, 1 << 4);
    impl_mods_fn!(dt, 1 << 6);
    impl_mods_fn!(rx, 1 << 7);
    impl_mods_fn!(ht, 1 << 8);
    impl_mods_fn!(fl, 1 << 10);
    impl_mods_fn!(so, 1 << 12);
    impl_mods_fn!(bl, false);
    impl_mods_fn!(tc, false);

    fn no_slider_head_acc(&self, lazer: bool) -> bool {
        !lazer
    }

    fn reflection(&self) -> Reflection {
        if self.hr() {
            Reflection::Vertical
        } else {
            Reflection::None
        }
    }
}

macro_rules! impl_has_mod {
    ( $( $fn:ident: $is_legacy:tt $name:ident, )* ) => {
        impl Mods for GameMods {
            $(
                fn $fn(&self) -> bool {
                    match self {
                        Self::Lazer(ref mods) => {
                            mods.contains_intermode(GameModIntermode::$name)
                        },
                        Self::Intermode(ref mods) => {
                            mods.contains(GameModIntermode::$name)
                        },
                        Self::Legacy(_mods) => {
                            impl_has_mod!(LEGACY $is_legacy $name _mods)
                        },
                    }
                }
            )*

            fn no_slider_head_acc(&self, lazer: bool) -> bool {
                match self {
                    Self::Lazer(ref mods) => mods
                        .iter()
                        .find_map(|m| match m {
                            GameMod::ClassicOsu(cl) => Some(cl.no_slider_head_accuracy.unwrap_or(true)),
                            _ => None,
                        })
                        .unwrap_or(!lazer),
                    Self::Intermode(ref mods) => mods.contains(GameModIntermode::Classic) || !lazer,
                    Self::Legacy(_) => !lazer,
                }
            }

            fn reflection(&self) -> Reflection {
                match self {
                    Self::Lazer(ref mods) => {
                        if mods.contains_intermode(GameModIntermode::HardRock) {
                            return Reflection::Vertical;
                        }

                        mods.iter()
                            .find_map(|m| match m {
                                GameMod::MirrorOsu(mr) => match mr.reflection.as_deref() {
                                    None => Some(Reflection::Horizontal),
                                    Some("1") => Some(Reflection::Vertical),
                                    Some("2") => Some(Reflection::Both),
                                    Some(_) => Some(Reflection::None),
                                },
                                GameMod::MirrorCatch(_) => Some(Reflection::Horizontal),
                                _ => None,
                            })
                            .unwrap_or(Reflection::None)
                    }
                    Self::Intermode(ref mods) => {
                        if mods.contains(GameModIntermode::HardRock) {
                            Reflection::Vertical
                        } else {
                            Reflection::None
                        }
                    }
                    Self::Legacy(mods) => {
                        if mods.contains(GameModsLegacy::HardRock) {
                            Reflection::Vertical
                        } else {
                            Reflection::None
                        }
                    }
                }
            }
        }
    };

    ( LEGACY + $name:ident $mods:ident ) => {
        $mods.contains(GameModsLegacy::$name)
    };

    ( LEGACY - $name:ident $mods:ident ) => {
        false
    };
}

impl_has_mod! {
    nf: + NoFail,
    ez: + Easy,
    td: + TouchDevice,
    hd: + Hidden,
    hr: + HardRock,
    dt: + DoubleTime,
    ht: + HalfTime,
    rx: + Relax,
    fl: + Flashlight,
    so: + SpunOut,
    bl: - Blinds,
    tc: - Traceable,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reflection {
    None,
    Vertical,
    Horizontal,
    Both,
}
