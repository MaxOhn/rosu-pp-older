use std::cmp;

use rosu_pp::{
    any::{
        hitresult_generator::{Fast, IgnoreAccuracy},
        HitResultGenerator,
    },
    taiko::TaikoHitResults,
};

use crate::taiko_2025::{performance::InspectTaikoPerformance, Taiko25};

impl HitResultGenerator<Taiko25> for Fast {
    fn generate_hitresults(inspect: InspectTaikoPerformance<'_>) -> TaikoHitResults {
        let Some(acc) = inspect.acc else {
            return <IgnoreAccuracy as HitResultGenerator<Taiko25>>::generate_hitresults(inspect);
        };

        let total_hits = inspect.total_hits();
        let misses = inspect.misses();
        let remain = total_hits - misses;

        let (n300, n100) = match (inspect.n300, inspect.n100) {
            (Some(n300), Some(n100)) => {
                let n300 = cmp::min(n300, remain);
                let n100 = cmp::min(n100, remain - n300);

                (n300, n100)
            }
            (Some(n300), None) => {
                let n300 = cmp::min(n300, remain);
                let n100 = remain - n300;

                (n300, n100)
            }
            (None, Some(n100)) => {
                let n100 = cmp::min(n100, remain);
                let n300 = remain - n100;

                (n300, n100)
            }
            (None, None) => {
                if remain == 0 {
                    return TaikoHitResults {
                        n300: 0,
                        n100: 0,
                        misses,
                    };
                }

                // acc = (2*n300 + n100) / (2*total_hits)
                // Simplify by multiplying by total_hits:
                // acc * (2*total_hits) = 2*n300 + n100

                let target_total = f64::round_ties_even(acc * f64::from(2 * total_hits)) as u32;

                // Start by assuming every non-miss is an n100
                // delta is how much we need to increase from the baseline (all n100s)
                let baseline = remain;
                let delta = target_total.saturating_sub(baseline);

                // Each n300 increases by 1 (2-1)
                // delta = 1*n300

                let n300 = cmp::min(remain, delta);
                let n100 = remain - n300;

                (n300, n100)
            }
        };

        TaikoHitResults { n300, n100, misses }
    }
}
