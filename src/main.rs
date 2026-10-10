fn main() {
    aige::run(
        neon_clash::build_game,
        std::path::Path::new("assets"),
        aige::WindowOptions {
            title: "NEON CLASH | AIGE".to_owned(),
            width: 1920,
            height: 1080,
            ..Default::default()
        },
    );
}
