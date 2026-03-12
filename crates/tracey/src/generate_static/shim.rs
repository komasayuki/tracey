pub(super) fn runtime_prelude() -> &'static str {
    concat!(
        include_str!("shim_core.js"),
        "\n",
        include_str!("shim_api.js")
    )
}
