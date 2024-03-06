use std::slice::Iter;

use rosu_pp::{
    model::hit_object::{HitObject, HitSoundType},
    Beatmap,
};

#[derive(Copy, Clone, Debug)]
pub(crate) struct TaikoObject<'h> {
    pub(crate) h: &'h HitObject,
    pub(crate) sound: HitSoundType,
}

pub(crate) trait IntoTaikoObjectIter {
    fn taiko_objects(&self) -> TaikoObjectIter<'_>;
}

#[derive(Clone, Debug)]
pub(crate) struct TaikoObjectIter<'m> {
    hit_objects: Iter<'m, HitObject>,
    sounds: Iter<'m, HitSoundType>,
}

impl IntoTaikoObjectIter for Beatmap {
    #[inline]
    fn taiko_objects(&self) -> TaikoObjectIter<'_> {
        TaikoObjectIter {
            hit_objects: self.hit_objects.iter(),
            sounds: self.hit_sounds.iter(),
        }
    }
}

impl<'m> Iterator for TaikoObjectIter<'m> {
    type Item = TaikoObject<'m>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        Some(TaikoObject {
            h: self.hit_objects.next()?,
            sound: *self.sounds.next()?,
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.hit_objects.size_hint()
    }
}

impl ExactSizeIterator for TaikoObjectIter<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.hit_objects.len()
    }
}
