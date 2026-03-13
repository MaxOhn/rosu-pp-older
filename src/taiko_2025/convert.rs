use rosu_map::section::hit_objects::hit_samples::HitSoundType;
use rosu_pp::Beatmap;

use crate::util::random;

pub(super) fn apply_random_to_beatmap(map: &mut Beatmap, seed: i32) {
    let mut rng = random::csharp::Random::new(seed);

    for (h, s) in map.hit_objects.iter().zip(map.hit_sounds.iter_mut()) {
        if !h.is_circle() {
            continue;
        }

        if rng.next_max(2) == 0 {
            // Center
            *s &= !(HitSoundType::CLAP | HitSoundType::WHISTLE);
        } else {
            // Rim
            *s = HitSoundType::from(u8::from(*s) | HitSoundType::CLAP);
        }
    }
}
