pub(super) struct StaticAsset {
    pub relative_path: &'static str,
    pub bytes: &'static [u8],
}

pub(super) const VENDOR_ASSETS: &[StaticAsset] = &[
    StaticAsset {
        relative_path: "vendor/NOTICE.txt",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/NOTICE.txt"),
    },
    StaticAsset {
        relative_path: "vendor/arborium/base.css",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/arborium/base.css"),
    },
    StaticAsset {
        relative_path: "vendor/arborium/kanagawa-dragon.css",
        bytes: include_bytes!(
            "../bridge/http/dashboard/public/vendor/arborium/kanagawa-dragon.css"
        ),
    },
    StaticAsset {
        relative_path: "vendor/arborium/github-light.css",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/arborium/github-light.css"),
    },
    StaticAsset {
        relative_path: "vendor/devicon/devicon.min.css",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/devicon/devicon.min.css"),
    },
    StaticAsset {
        relative_path: "vendor/devicon/fonts/devicon.eot",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/devicon/fonts/devicon.eot"),
    },
    StaticAsset {
        relative_path: "vendor/devicon/fonts/devicon.ttf",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/devicon/fonts/devicon.ttf"),
    },
    StaticAsset {
        relative_path: "vendor/devicon/fonts/devicon.woff",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/devicon/fonts/devicon.woff"),
    },
    StaticAsset {
        relative_path: "vendor/devicon/fonts/devicon.svg",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/devicon/fonts/devicon.svg"),
    },
    StaticAsset {
        relative_path: "vendor/lucide/lucide.min.js",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/lucide/lucide.min.js"),
    },
    StaticAsset {
        relative_path: "vendor/recursive/recursive.css",
        bytes: include_bytes!("../bridge/http/dashboard/public/vendor/recursive/recursive.css"),
    },
    StaticAsset {
        relative_path: "vendor/recursive/files/recursive-cyrillic-ext-full-normal.woff2",
        bytes: include_bytes!(
            "../bridge/http/dashboard/public/vendor/recursive/files/recursive-cyrillic-ext-full-normal.woff2"
        ),
    },
    StaticAsset {
        relative_path: "vendor/recursive/files/recursive-vietnamese-full-normal.woff2",
        bytes: include_bytes!(
            "../bridge/http/dashboard/public/vendor/recursive/files/recursive-vietnamese-full-normal.woff2"
        ),
    },
    StaticAsset {
        relative_path: "vendor/recursive/files/recursive-latin-ext-full-normal.woff2",
        bytes: include_bytes!(
            "../bridge/http/dashboard/public/vendor/recursive/files/recursive-latin-ext-full-normal.woff2"
        ),
    },
    StaticAsset {
        relative_path: "vendor/recursive/files/recursive-latin-full-normal.woff2",
        bytes: include_bytes!(
            "../bridge/http/dashboard/public/vendor/recursive/files/recursive-latin-full-normal.woff2"
        ),
    },
];
