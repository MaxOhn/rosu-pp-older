use std::{
    borrow::Cow,
    fmt::{Debug, Formatter, Result as FmtResult},
};

use rosu_pp::{model::mode::ConvertError, Beatmap};

pub enum MapOrAttrs<'map, A> {
    Map(Cow<'map, Beatmap>),
    Attrs(A),
}

impl<A> MapOrAttrs<'_, A> {
    pub fn insert_attrs<F>(&mut self, attrs_fn: F) -> Result<(), ConvertError>
    where
        F: FnOnce() -> Result<A, ConvertError>,
    {
        if let Self::Map(map) = self {
            *self = Self::Attrs(attrs_fn()?)
        }

        Ok(())
    }

    /// Get a reference to the attributes.
    ///
    /// # Safety
    /// Caller must ensure that this [`MapOrAttrs`] contains attributes.
    pub const unsafe fn get_attrs(&self) -> &A {
        // Returning an immutable reference while requiring a mutable reference
        // as argument, unfortunately, makes it impossible to pass another
        // mutable reference later on. Instead we split it up into two
        // functions: first `insert_attrs` and then `get_attrs`.
        match self {
            Self::Attrs(attrs) => attrs,
            // SAFETY: Up to the caller to uphold
            Self::Map(_) => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

impl<A: Clone> Clone for MapOrAttrs<'_, A> {
    fn clone(&self) -> Self {
        match self {
            Self::Map(converted) => Self::Map(converted.clone()),
            Self::Attrs(attrs) => Self::Attrs(attrs.clone()),
        }
    }
}

impl<A: Debug> Debug for MapOrAttrs<'_, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Map(converted) => f.debug_tuple("Map").field(converted).finish(),
            Self::Attrs(attrs) => f.debug_tuple("Attrs").field(attrs).finish(),
        }
    }
}

impl<A> PartialEq for MapOrAttrs<'_, A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Map(a), Self::Map(b)) => a == b,
            (Self::Attrs(a), Self::Attrs(b)) => a == b,
            _ => false,
        }
    }
}

impl<'map, A> From<&'map Beatmap> for MapOrAttrs<'map, A> {
    fn from(map: &'map Beatmap) -> Self {
        Self::Map(Cow::Borrowed(map))
    }
}

impl<A> From<Beatmap> for MapOrAttrs<'_, A> {
    fn from(map: Beatmap) -> Self {
        Self::Map(Cow::Owned(map))
    }
}
