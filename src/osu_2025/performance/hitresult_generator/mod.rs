use rosu_pp::osu::OsuScoreOrigin;

mod fast;
mod ignore_acc;

fn tick_scores(
    origin: &OsuScoreOrigin,
    large_tick_hits: u32,
    small_tick_hits: u32,
    slider_end_hits: u32,
) -> (u32, u32) {
    match origin {
        OsuScoreOrigin::Stable => (0, 0),
        OsuScoreOrigin::WithSliderAcc {
            max_large_ticks,
            max_slider_ends,
        } => (
            150 * slider_end_hits + 30 * large_tick_hits,
            150 * max_slider_ends + 30 * max_large_ticks,
        ),
        OsuScoreOrigin::WithoutSliderAcc {
            max_large_ticks,
            max_small_ticks,
        } => (
            30 * large_tick_hits + 10 * small_tick_hits,
            30 * max_large_ticks + 10 * max_small_ticks,
        ),
    }
}
