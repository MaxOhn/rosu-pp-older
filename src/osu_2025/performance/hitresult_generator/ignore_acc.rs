use std::{cmp, mem};

use rosu_pp::{
    any::{hitresult_generator::IgnoreAccuracy, HitResultGenerator, HitResultPriority},
    osu::OsuHitResults,
};

use crate::osu_2025::{performance::InspectOsuPerformance, Osu25};

impl HitResultGenerator<Osu25> for IgnoreAccuracy {
    fn generate_hitresults(inspect: InspectOsuPerformance<'_>) -> OsuHitResults {
        let (slider_end_hits, large_tick_hits, small_tick_hits) = inspect.tick_hits();

        let total_hits = inspect.total_hits();
        let misses = inspect.misses();
        let mut remain = total_hits - misses;

        // Helper to assign a specified value
        let mut assign_specified = |specified: Option<u32>| -> Option<u32> {
            let assigned = cmp::min(specified?, remain);
            remain -= assigned;

            Some(assigned)
        };

        let (n300, n100, n50) = match inspect.hitresult_priority {
            HitResultPriority::BestCase => {
                // First pass: assign specified values in priority order
                let n300 = assign_specified(inspect.n300);
                let n100 = assign_specified(inspect.n100);
                let n50 = assign_specified(inspect.n50);

                // Second pass: fill first unspecified with remainder
                let mut n300 = n300.unwrap_or_else(|| mem::replace(&mut remain, 0));
                let n100 = n100.unwrap_or_else(|| mem::replace(&mut remain, 0));
                let n50 = n50.unwrap_or_else(|| mem::replace(&mut remain, 0));

                if remain > 0 {
                    n300 += remain;
                }

                (n300, n100, n50)
            }
            HitResultPriority::WorstCase => {
                // First pass: assign specified values in priority order (worst to best)
                let n50 = assign_specified(inspect.n50);
                let n100 = assign_specified(inspect.n100);
                let n300 = assign_specified(inspect.n300);

                // Second pass: fill first unspecified with remainder
                let mut n50 = n50.unwrap_or_else(|| mem::replace(&mut remain, 0));
                let n100 = n100.unwrap_or_else(|| mem::replace(&mut remain, 0));
                let n300 = n300.unwrap_or_else(|| mem::replace(&mut remain, 0));

                if remain > 0 {
                    n50 += remain;
                }

                (n300, n100, n50)
            }
        };

        OsuHitResults {
            large_tick_hits,
            small_tick_hits,
            slider_end_hits,
            n300,
            n100,
            n50,
            misses,
        }
    }
}
