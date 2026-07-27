use trk_core::{
    BitcrusherSpec, ChorusSpec, CompressorSpec, DelaySpec, DriveSpec, EffectDevice, FilterSpec,
    FlangerSpec, GateSpec, LimiterSpec, PhaserSpec, ReverbSpec,
};

pub(super) fn effect_device_from_name(id: u32, name: &str) -> Option<EffectDevice> {
    let normalized = name.to_ascii_lowercase();
    if normalized.contains("gain") || normalized.contains("gainer") || normalized.contains("volume")
    {
        Some(EffectDevice::gain(id, 1.0))
    } else if normalized.contains("pan") {
        Some(EffectDevice::pan(id, 0.0))
    } else if normalized.contains("filter") {
        Some(EffectDevice::filter(id, FilterSpec::default()))
    } else if normalized.contains("delay")
        || normalized.contains("echo")
        || normalized.contains("repeater")
    {
        Some(EffectDevice::delay(id, DelaySpec::default()))
    } else if normalized.contains("reverb") {
        Some(EffectDevice::reverb(id, ReverbSpec::default()))
    } else if normalized.contains("drive")
        || normalized.contains("distortion")
        || normalized.contains("overdrive")
    {
        Some(EffectDevice::drive(id, DriveSpec::default()))
    } else if normalized.contains("bitcrush") || normalized.contains("lo-fi") {
        Some(EffectDevice::bitcrusher(id, BitcrusherSpec::default()))
    } else if normalized.contains("chorus") {
        Some(EffectDevice::chorus(id, ChorusSpec::default()))
    } else if normalized.contains("flanger") {
        Some(EffectDevice::flanger(id, FlangerSpec::default()))
    } else if normalized.contains("phaser") {
        Some(EffectDevice::phaser(id, PhaserSpec::default()))
    } else if normalized.contains("compressor") {
        Some(EffectDevice::compressor(id, CompressorSpec::default()))
    } else if normalized.contains("gate") {
        Some(EffectDevice::gate(id, GateSpec::default()))
    } else if normalized.contains("limiter") || normalized.contains("maximizer") {
        Some(EffectDevice::limiter(id, LimiterSpec::default()))
    } else {
        None
    }
}

pub(super) fn is_supported_native_device(device: &str) -> bool {
    effect_device_from_name(0, device).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trk_core::EffectDeviceKind;

    #[test]
    fn maps_supported_renoise_device_names_to_native_effects() {
        assert!(matches!(
            effect_device_from_name(1, "Filter").expect("filter").kind,
            EffectDeviceKind::Filter { .. }
        ));
        assert!(matches!(
            effect_device_from_name(2, "Delay").expect("delay").kind,
            EffectDeviceKind::Delay { .. }
        ));
        assert!(matches!(
            effect_device_from_name(3, "Chorus").expect("chorus").kind,
            EffectDeviceKind::Chorus { .. }
        ));
    }
}
