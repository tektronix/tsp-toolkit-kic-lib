use std::{fmt::Display, str::FromStr};

use crate::InstrumentError;

#[allow(non_camel_case_types)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, serde::Serialize)]
pub enum Model {
    //26xx
    _2601,
    _2602,
    _2611,
    _2612,
    _2635,
    _2636,
    _2601A,
    _2602A,
    _2611A,
    _2612A,
    _2635A,
    _2636A,
    _2651A,
    _2657A,
    _2601B,
    _2601B_PULSE,
    _2602B,
    _2606B,
    _2611B,
    _2612B,
    _2635B,
    _2636B,
    _2604B,
    _2614B,
    _2634B,
    _2601B_L,
    _2602B_L,
    _2611B_L,
    _2612B_L,
    _2635B_L,
    _2636B_L,
    _2604B_L,
    _2614B_L,
    _2634B_L,

    // 3706 or 70xB
    _3706,
    _3706_SNFP,
    _3706_S,
    _3706_NFP,
    _3706A,
    _3706A_SNFP,
    _3706A_S,
    _3706A_NFP,
    _707B,
    _708B,

    // TTI
    _2450,
    _2470,
    _DMM7510,
    _2460,
    _2461,
    _2461_SYS,
    DMM7512,
    DMM6500,
    DAQ6510,

    // Modular Platform
    Mp5103(String),

    // Anything else
    Other(String),
}

impl Model {
    #[must_use]
    pub const fn is_tti(&self) -> bool {
        matches!(
            self,
            Self::_2450
                | Self::_2470
                | Self::_DMM7510
                | Self::_2460
                | Self::_2461
                | Self::_2461_SYS
                | Self::DMM7512
                | Self::DMM6500
                | Self::DAQ6510
        )
    }

    #[must_use]
    pub const fn is_mp(&self) -> bool {
        matches!(self, Self::Mp5103(_))
    }

    #[must_use]
    pub const fn is_3700_70x(&self) -> bool {
        matches!(
            self,
            Self::_3706
                | Self::_3706_SNFP
                | Self::_3706_S
                | Self::_3706_NFP
                | Self::_3706A
                | Self::_3706A_SNFP
                | Self::_3706A_S
                | Self::_3706A_NFP
                | Self::_707B
                | Self::_708B
        )
    }

    #[must_use]
    pub const fn is_2600(&self) -> bool {
        matches!(
            self,
            Self::_2601
                | Self::_2602
                | Self::_2611
                | Self::_2612
                | Self::_2635
                | Self::_2636
                | Self::_2601A
                | Self::_2602A
                | Self::_2611A
                | Self::_2612A
                | Self::_2635A
                | Self::_2636A
                | Self::_2651A
                | Self::_2657A
                | Self::_2601B
                | Self::_2601B_PULSE
                | Self::_2602B
                | Self::_2606B
                | Self::_2611B
                | Self::_2612B
                | Self::_2635B
                | Self::_2636B
                | Self::_2604B
                | Self::_2614B
                | Self::_2634B
                | Self::_2601B_L
                | Self::_2602B_L
                | Self::_2611B_L
                | Self::_2612B_L
                | Self::_2635B_L
                | Self::_2636B_L
                | Self::_2604B_L
                | Self::_2614B_L
                | Self::_2634B_L
        )
    }

    #[must_use]
    pub const fn is_other(&self) -> bool {
        matches!(self, Self::Other(_))
    }
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let model_str = match self {
            //26xx
            Self::_2601 => "2601",
            Self::_2602 => "2602",
            Self::_2611 => "2611",
            Self::_2612 => "2612",
            Self::_2635 => "2635",
            Self::_2636 => "2636",
            Self::_2601A => "2601A",
            Self::_2602A => "2602A",
            Self::_2611A => "2611A",
            Self::_2612A => "2612A",
            Self::_2635A => "2635A",
            Self::_2636A => "2636A",
            Self::_2651A => "2651A",
            Self::_2657A => "2657A",
            Self::_2601B => "2601B",
            Self::_2601B_PULSE => "2601B-PULSE",
            Self::_2602B => "2602B",
            Self::_2606B => "2606B",
            Self::_2611B => "2611B",
            Self::_2612B => "2612B",
            Self::_2635B => "2635B",
            Self::_2636B => "2636B",
            Self::_2604B => "2604B",
            Self::_2614B => "2614B",
            Self::_2634B => "2634B",
            Self::_2601B_L => "2601B-L",
            Self::_2602B_L => "2602B-L",
            Self::_2611B_L => "2611B-L",
            Self::_2612B_L => "2612B-L",
            Self::_2635B_L => "2635B-L",
            Self::_2636B_L => "2636B-L",
            Self::_2604B_L => "2604B-L",
            Self::_2614B_L => "2614B-L",
            Self::_2634B_L => "2634B-L",

            // 3706 or 70xB
            Self::_3706 => "3706",
            Self::_3706_SNFP => "3706-SNFP",
            Self::_3706_S => "3706-S",
            Self::_3706_NFP => "3706-NFP",
            Self::_3706A => "3706A",
            Self::_3706A_SNFP => "3706A-SNFP",
            Self::_3706A_S => "3706A-S",
            Self::_3706A_NFP => "3706A-NFP",
            Self::_707B => "707B",
            Self::_708B => "708B",

            // TTI
            Self::_2450 => "2450",
            Self::_2470 => "2470",
            Self::_DMM7510 => "DMM7510-",
            Self::_2460 => "2460",
            Self::_2461 => "2461",
            Self::_2461_SYS => "2461-SYS",
            Self::DMM7512 => "DMM7512",
            Self::DMM6500 => "DMM6500",
            Self::DAQ6510 => "DAQ6510",

            // Modular Platform
            Self::Mp5103(s) | Self::Other(s) => s,
        };

        write!(f, "{model_str}")
    }
}

impl FromStr for Model {
    type Err = InstrumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            //26xx
            "2601" => Self::_2601,
            "2602" => Self::_2602,
            "2611" => Self::_2611,
            "2612" => Self::_2612,
            "2635" => Self::_2635,
            "2636" => Self::_2636,
            "2601A" => Self::_2601A,
            "2602A" => Self::_2602A,
            "2611A" => Self::_2611A,
            "2612A" => Self::_2612A,
            "2635A" => Self::_2635A,
            "2636A" => Self::_2636A,
            "2651A" => Self::_2651A,
            "2657A" => Self::_2657A,
            "2601B" => Self::_2601B,
            "2601B-PULSE" => Self::_2601B_PULSE,
            "2602B" => Self::_2602B,
            "2606B" => Self::_2606B,
            "2611B" => Self::_2611B,
            "2612B" => Self::_2612B,
            "2635B" => Self::_2635B,
            "2636B" => Self::_2636B,
            "2604B" => Self::_2604B,
            "2614B" => Self::_2614B,
            "2634B" => Self::_2634B,
            "2601B-L" => Self::_2601B_L,
            "2602B-L" => Self::_2602B_L,
            "2611B-L" => Self::_2611B_L,
            "2612B-L" => Self::_2612B_L,
            "2635B-L" => Self::_2635B_L,
            "2636B-L" => Self::_2636B_L,
            "2604B-L" => Self::_2604B_L,
            "2614B-L" => Self::_2614B_L,
            "2634B-L" => Self::_2634B_L,

            // 3706 or 70xB
            "3706" => Self::_3706,
            "3706-SNFP" => Self::_3706_SNFP,
            "3706-S" => Self::_3706_S,
            "3706-NFP" => Self::_3706_NFP,
            "3706A" => Self::_3706A,
            "3706A-SNFP" => Self::_3706A_SNFP,
            "3706A-S" => Self::_3706A_S,
            "3706A-NFP" => Self::_3706A_NFP,
            "707B" => Self::_707B,
            "708B" => Self::_708B,

            // TTI
            "2450" => Self::_2450,
            "2470" => Self::_2470,
            "DMM7510-" => Self::_DMM7510,
            "2460" => Self::_2460,
            "2461" => Self::_2461,
            "2461-SYS" => Self::_2461_SYS,
            "DMM7512" => Self::DMM7512,
            "DMM6500" => Self::DMM6500,
            "DAQ6510" => Self::DAQ6510,

            // Modular Platform
            "MP5103" | "VERSATEST-300" | "VERSATEST-600" | "TSPop" | "TSP" => {
                Self::Mp5103(s.to_string())
            }

            //Other
            _ => Self::Other(s.to_string()),
        })
    }
}
