use std::cmp;

use rosu_pp::{
    any::{
        hitresult_generator::{Fast, IgnoreAccuracy},
        HitResultGenerator, HitResultPriority,
    },
    osu::OsuHitResults,
};

use crate::osu_2025::{performance::InspectOsuPerformance, Osu25};

impl HitResultGenerator<Osu25> for Fast {
    fn generate_hitresults(inspect: InspectOsuPerformance<'_>) -> OsuHitResults {
        let Some(acc) = inspect.acc else {
            return <IgnoreAccuracy as HitResultGenerator<Osu25>>::generate_hitresults(inspect);
        };

        let large_tick_hits = inspect.large_tick_hits.unwrap_or(0);
        let small_tick_hits = inspect.small_tick_hits.unwrap_or(0);
        let slider_end_hits = inspect.slider_end_hits.unwrap_or(0);

        let total_hits = inspect.total_hits();
        let misses = inspect.misses();
        let remain = total_hits - misses;
        let origin = inspect.origin();

        if remain == 0 {
            return OsuHitResults {
                large_tick_hits,
                small_tick_hits,
                slider_end_hits,
                n300: 0,
                n100: 0,
                n50: 0,
                misses,
            };
        }

        let (tick_score, tick_max) =
            super::tick_scores(&origin, large_tick_hits, small_tick_hits, slider_end_hits);

        let prelim_300 = inspect.n300.map_or(0, |n| cmp::min(n, remain));
        let prelim_100 = inspect.n100.map_or(0, |n| cmp::min(n, remain - prelim_300));
        let prelim_50 = inspect
            .n50
            .map_or(0, |n| cmp::min(n, remain - prelim_300 - prelim_100));

        let (n300, n100, n50) = match (inspect.n300, inspect.n100, inspect.n50) {
            // None missing
            (Some(_), Some(_), Some(_)) => (prelim_300, prelim_100, prelim_50),

            // Only one missing
            (Some(_), Some(_), None) => (prelim_300, prelim_100, remain - prelim_300 - prelim_100),
            (Some(_), None, Some(_)) => (prelim_300, remain - prelim_300 - prelim_50, prelim_50),
            (None, Some(_), Some(_)) => (remain - prelim_100 - prelim_50, prelim_100, prelim_50),

            // Two or three missing - use Fast algorithm
            _ => {
                // acc = (300*n300 + 100*n100 + 50*n50 + tick_score) / (300*total_hits + tick_max)
                // Simplify by dividing by 50: (reducing risk of overflow)
                // acc = (6*n300 + 2*n100 + n50 + tick_score/50) / (6*total_hits + tick_max/50)

                let numerator = f64::from(6 * prelim_300 + 2 * prelim_100 + prelim_50)
                    + f64::from(tick_score) / 50.0;

                let denominator = f64::from(6 * total_hits) + f64::from(tick_max) / 50.0;

                let target_total =
                    f64::round_ties_even((acc * denominator - numerator).max(0.0)) as u32;

                // Start by assuming every non-miss is an n50
                // delta is how much we need to increase from the baseline (all n50s)
                let baseline = remain - prelim_300 - prelim_100 - prelim_50;
                let mut delta = target_total.saturating_sub(baseline);

                // Each n300 increases by 5 (6-1), each n100 increases by 1 (2-1)
                // delta = 5*n300 + 1*n100

                let n300 = cmp::min(
                    remain - prelim_100 - prelim_50,
                    inspect.n300.unwrap_or(delta / 5),
                );

                if inspect.n300.is_none() {
                    delta = delta.saturating_sub(5 * n300);
                }

                let n100 = cmp::min(remain - n300 - prelim_50, inspect.n100.unwrap_or(delta));
                let n50 = cmp::min(remain - n300 - n100, inspect.n50.unwrap_or(remain));

                (n300, n100, n50)
            }
        };

        let mut hitresults = OsuHitResults {
            large_tick_hits,
            small_tick_hits,
            slider_end_hits,
            n300,
            n100,
            n50,
            misses,
        };

        if hitresults.total_hits() < total_hits {
            let left = total_hits - hitresults.total_hits();

            match inspect.hitresult_priority {
                HitResultPriority::BestCase => match inspect {
                    InspectOsuPerformance { n300: None, .. } => hitresults.n300 += left,
                    InspectOsuPerformance { n100: None, .. } => hitresults.n100 += left,
                    _ => hitresults.n50 += left,
                },
                HitResultPriority::WorstCase => match inspect {
                    InspectOsuPerformance { n50: None, .. } => hitresults.n50 += left,
                    InspectOsuPerformance { n100: None, .. } => hitresults.n100 += left,
                    _ => hitresults.n300 += left,
                },
            }
        }

        hitresults
    }
}
