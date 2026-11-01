//! One font inventory for both native adapters; licensing lives beside the assets.
pub const BUNDLED_FONTS: &[(&str, &[u8])] = &[
    (
        "Fraunces 72pt Soft",
        include_bytes!("../../../../assets/fonts/fraunces/Fraunces72ptSoft-SemiBold.ttf"),
    ),
    (
        "Fraunces",
        include_bytes!("../../../../assets/fonts/fraunces/Fraunces-Variable.ttf"),
    ),
    (
        "Literata",
        include_bytes!("../../../../assets/fonts/literata/Literata-Variable.ttf"),
    ),
    (
        "Literata",
        include_bytes!("../../../../assets/fonts/literata/Literata-Italic-Variable.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Italic.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Medium.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../../assets/fonts/ibm-plex-sans/IBMPlexSans-MediumItalic.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf"),
    ),
    (
        "IBM Plex Mono",
        include_bytes!("../../../../assets/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf"),
    ),
    (
        "IBM Plex Mono",
        include_bytes!("../../../../assets/fonts/ibm-plex-mono/IBMPlexMono-Italic.ttf"),
    ),
    (
        "IBM Plex Mono",
        include_bytes!("../../../../assets/fonts/ibm-plex-mono/IBMPlexMono-Medium.ttf"),
    ),
];
