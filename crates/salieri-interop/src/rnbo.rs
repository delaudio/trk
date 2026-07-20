#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RnboWorkflowKind {
    CppSourceExport,
    WebExportJson,
    CloudPluginBinaryExport,
    SalieriToRnboWrapper,
    CoreRuntimeStateImport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RnboRecommendation {
    ViableBehindNativeBoundary,
    ViableForWebExportBoundary,
    DeferUntilSchemaAdr,
    NotRecommended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RnboArtifactKind {
    CppSource,
    WebExportJson,
    ParameterManifest,
    OpaqueRuntimeState,
    PluginBinary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RnboWorkflowAssessment {
    pub kind: RnboWorkflowKind,
    pub recommendation: RnboRecommendation,
    pub requires_max_or_rnbo_license: bool,
    pub depends_on_cloud_export: bool,
    pub can_work_offline_after_export: bool,
    pub may_store_project_state_now: bool,
    pub notes: Vec<&'static str>,
}

pub fn assess_rnbo_workflow(kind: RnboWorkflowKind) -> RnboWorkflowAssessment {
    match kind {
        RnboWorkflowKind::CppSourceExport => RnboWorkflowAssessment {
            kind,
            recommendation: RnboRecommendation::ViableBehindNativeBoundary,
            requires_max_or_rnbo_license: true,
            depends_on_cloud_export: false,
            can_work_offline_after_export: true,
            may_store_project_state_now: false,
            notes: vec![
                "review generated C++ and wrap it through the #115 native boundary",
                "store only Salieri descriptors and parameters until a schema ADR exists",
            ],
        },
        RnboWorkflowKind::WebExportJson => RnboWorkflowAssessment {
            kind,
            recommendation: RnboRecommendation::ViableForWebExportBoundary,
            requires_max_or_rnbo_license: true,
            depends_on_cloud_export: false,
            can_work_offline_after_export: true,
            may_store_project_state_now: false,
            notes: vec![
                "map RNBO Web Export artifacts to the #117 browser export boundary",
                "do not use @rnbo/js concepts in salieri-core",
            ],
        },
        RnboWorkflowKind::CloudPluginBinaryExport => RnboWorkflowAssessment {
            kind,
            recommendation: RnboRecommendation::NotRecommended,
            requires_max_or_rnbo_license: true,
            depends_on_cloud_export: true,
            can_work_offline_after_export: false,
            may_store_project_state_now: false,
            notes: vec![
                "cloud export and binary distribution constraints do not fit Salieri core",
                "plugin binary export is separate from Salieri native module data",
            ],
        },
        RnboWorkflowKind::SalieriToRnboWrapper => RnboWorkflowAssessment {
            kind,
            recommendation: RnboRecommendation::DeferUntilSchemaAdr,
            requires_max_or_rnbo_license: true,
            depends_on_cloud_export: true,
            can_work_offline_after_export: false,
            may_store_project_state_now: false,
            notes: vec![
                "requires an ADR for schema, ownership, and reverse mapping",
                "not needed for native runtime or browser export foundations",
            ],
        },
        RnboWorkflowKind::CoreRuntimeStateImport => RnboWorkflowAssessment {
            kind,
            recommendation: RnboRecommendation::NotRecommended,
            requires_max_or_rnbo_license: true,
            depends_on_cloud_export: false,
            can_work_offline_after_export: false,
            may_store_project_state_now: false,
            notes: vec![
                "opaque RNBO state must not be stored in project files",
                "use explicit parameter manifests instead",
            ],
        },
    }
}

pub fn rnbo_project_state_policy(artifact: RnboArtifactKind) -> bool {
    matches!(
        artifact,
        RnboArtifactKind::CppSource
            | RnboArtifactKind::WebExportJson
            | RnboArtifactKind::ParameterManifest
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rnbo_cpp_source_is_viable_only_behind_native_boundary() {
        let assessment = assess_rnbo_workflow(RnboWorkflowKind::CppSourceExport);

        assert_eq!(
            assessment.recommendation,
            RnboRecommendation::ViableBehindNativeBoundary
        );
        assert!(assessment.requires_max_or_rnbo_license);
        assert!(!assessment.depends_on_cloud_export);
        assert!(assessment.can_work_offline_after_export);
        assert!(!assessment.may_store_project_state_now);
    }

    #[test]
    fn rnbo_web_export_is_limited_to_web_boundary() {
        let assessment = assess_rnbo_workflow(RnboWorkflowKind::WebExportJson);

        assert_eq!(
            assessment.recommendation,
            RnboRecommendation::ViableForWebExportBoundary
        );
        assert!(assessment.can_work_offline_after_export);
        assert!(assessment
            .notes
            .iter()
            .any(|note| note.contains("#117 browser export")));
    }

    #[test]
    fn rnbo_cloud_plugin_exports_are_not_recommended_for_core() {
        let assessment = assess_rnbo_workflow(RnboWorkflowKind::CloudPluginBinaryExport);

        assert_eq!(
            assessment.recommendation,
            RnboRecommendation::NotRecommended
        );
        assert!(assessment.depends_on_cloud_export);
        assert!(!assessment.can_work_offline_after_export);
        assert!(!assessment.may_store_project_state_now);
    }

    #[test]
    fn rnbo_opaque_state_is_rejected_until_schema_adr() {
        assert!(rnbo_project_state_policy(RnboArtifactKind::CppSource));
        assert!(rnbo_project_state_policy(RnboArtifactKind::WebExportJson));
        assert!(rnbo_project_state_policy(
            RnboArtifactKind::ParameterManifest
        ));
        assert!(!rnbo_project_state_policy(
            RnboArtifactKind::OpaqueRuntimeState
        ));
        assert!(!rnbo_project_state_policy(RnboArtifactKind::PluginBinary));

        let assessment = assess_rnbo_workflow(RnboWorkflowKind::CoreRuntimeStateImport);
        assert_eq!(
            assessment.recommendation,
            RnboRecommendation::NotRecommended
        );
        assert!(!assessment.may_store_project_state_now);
    }
}
