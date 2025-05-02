use serde::{Deserialize, Serialize};
use serde_repr::*;
use ts_rs::TS;

pub mod advanced_display;
pub mod base_display;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DisplayCommand {
    /// 23.1. Get Base Display Parameter Setting
    GetBase(base_display::BaseParameterSetting),
    /// 23.2. Set Base Display Parameter Setting
    SetBase(base_display::BaseParameterSetting),
    /// 23.3. Get Advanced Display Parameter Setting
    GetAdvanced(advanced_display::AdvancedParameterSetting),
    /// 23.4. Set Advanced Display Parameter Setting
    SetAdvanced(advanced_display::AdvancedParameterSetting),
    /// 23.5. Base Display Parameter Restore
    RestoreBase,
    /// 23.6. Advanced Display Parameter Restore
    RestoreAdvanced,
}
