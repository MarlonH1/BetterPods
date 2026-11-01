//! AirPods models and what each one supports, ported from the Android `data/AirPods.kt`
//! so both apps show the same settings for the same hardware, like an iPhone does.

/// A feature that only some models have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    ListeningMode,
    ConversationAwareness,
    StemConfig,
    HeadGestures,
    LoudSoundReduction,
    Ppe,
    SleepDetection,
    HearingAid,
    AdaptiveAudio,
    AdaptiveVolume,
    SwipeForVolume,
    Hrm,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Model {
    pub name: &'static str,
    pub model_numbers: &'static [&'static str],
    pub capabilities: &'static [Capability],
    /// Over-ear: one battery, no case, no stems.
    pub headphones: bool,
}

impl Model {
    pub fn has(&self, c: Capability) -> bool {
        self.capabilities.contains(&c)
    }

    pub fn by_number(number: &str) -> Option<&'static Model> {
        MODELS.iter().find(|m| m.model_numbers.contains(&number))
    }
}

use Capability::*;

const PRO2: &[Capability] = &[
    ListeningMode,
    ConversationAwareness,
    StemConfig,
    LoudSoundReduction,
    SleepDetection,
    HearingAid,
    AdaptiveAudio,
    AdaptiveVolume,
    SwipeForVolume,
    HeadGestures,
];
const GEN5: &[Capability] =
    &[ListeningMode, ConversationAwareness, HeadGestures, AdaptiveAudio, SleepDetection, AdaptiveVolume, StemConfig];

pub static MODELS: &[Model] = &[
    Model { name: "AirPods (1. Generation)", model_numbers: &["A1523", "A1722"], capabilities: &[], headphones: false },
    Model { name: "AirPods (2. Generation)", model_numbers: &["A2032", "A2031"], capabilities: &[], headphones: false },
    Model {
        name: "AirPods (3. Generation)",
        model_numbers: &["A2565", "A2564"],
        capabilities: &[HeadGestures],
        headphones: false,
    },
    Model {
        name: "AirPods 4",
        model_numbers: &["A3053", "A3050", "A3054"],
        capabilities: &[HeadGestures, SleepDetection, AdaptiveVolume],
        headphones: false,
    },
    Model {
        name: "AirPods 4 mit aktiver Geräuschunterdrückung",
        model_numbers: &["A3056", "A3055", "A3057"],
        capabilities: GEN5,
        headphones: false,
    },
    Model { name: "AirPods 5", model_numbers: &["A3531", "A3532", "A3533"], capabilities: GEN5, headphones: false },
    Model {
        name: "AirPods 5 mit kabellosem Ladecase",
        model_numbers: &["A3439", "A3440", "A3441"],
        capabilities: &[
            ListeningMode,
            ConversationAwareness,
            HeadGestures,
            AdaptiveAudio,
            SleepDetection,
            AdaptiveVolume,
            StemConfig,
            SwipeForVolume,
        ],
        headphones: false,
    },
    Model {
        name: "AirPods Pro",
        model_numbers: &["A2084", "A2083"],
        capabilities: &[ListeningMode],
        headphones: false,
    },
    Model {
        name: "AirPods Pro 2 (Lightning)",
        model_numbers: &["A2931", "A2699", "A2698"],
        capabilities: PRO2,
        headphones: false,
    },
    Model {
        name: "AirPods Pro 2 (USB-C)",
        model_numbers: &["A3047", "A3048", "A3049"],
        capabilities: PRO2,
        headphones: false,
    },
    Model {
        name: "AirPods Pro 3",
        model_numbers: &["A3063", "A3064", "A3065"],
        capabilities: &[
            ListeningMode,
            ConversationAwareness,
            HeadGestures,
            StemConfig,
            LoudSoundReduction,
            Ppe,
            SleepDetection,
            HearingAid,
            AdaptiveAudio,
            AdaptiveVolume,
            SwipeForVolume,
            Hrm,
        ],
        headphones: false,
    },
    Model { name: "AirPods Max", model_numbers: &["A2096", "A3184"], capabilities: &[ListeningMode], headphones: true },
];
