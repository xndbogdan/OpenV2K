use super::*;
use crate::system_layout::{
    prepare_system_layout, HighSystemLayerReceipt, PreparedHighSystemLayouts, SystemLayoutOrigin,
    SystemLayoutOrigins, SystemLayoutRefreshError,
};

impl ResourceCache {
    /// Source-backed auxiliary insertion. Parsed/ad-hoc auxiliary callers
    /// retain add_auxiliary and deliberately have no raw high-tier receipt.
    pub(crate) fn add_auxiliary_with_layout_receipt(
        &mut self,
        state: LevelState,
        receipt: HighSystemLayerReceipt,
    ) {
        self.insert_auxiliary(state, Some(receipt));
    }

    pub fn system_layout_origin(&self, level: u32) -> Option<&SystemLayoutOrigin> {
        self.layers
            .iter()
            .rev()
            .find(|layer| layer.state.system_level == Some(level))?
            .system_layout_receipt
            .as_ref()
            .map(|receipt| &receipt.origin)
    }

    pub(crate) fn prepare_high_system_layouts(
        &self,
        candidates: [(v2k_formats::ovl::OvlFile, SystemLayoutOrigin); 2],
    ) -> Result<PreparedHighSystemLayouts, SystemLayoutRefreshError> {
        let mut prepared = Vec::with_capacity(2);
        for ((ovl, origin), system_level) in candidates.into_iter().zip([2, 3]) {
            let layer = self
                .layers
                .iter()
                .rev()
                .find(|layer| layer.state.system_level == Some(system_level))
                .ok_or(SystemLayoutRefreshError::ResidentUnavailable { system_level })?;
            let receipt = layer
                .system_layout_receipt
                .as_ref()
                .ok_or(SystemLayoutRefreshError::ResidentSourceUnproven { system_level })?;
            let data =
                layer
                    .state
                    .fixup_data
                    .as_ref()
                    .ok_or(SystemLayoutRefreshError::InvalidTable {
                        system_level,
                        section: 0,
                    })?;
            let points =
                layer
                    .state
                    .fixup_code
                    .as_ref()
                    .ok_or(SystemLayoutRefreshError::InvalidTable {
                        system_level,
                        section: 1,
                    })?;
            prepared.push(prepare_system_layout(
                system_level,
                &ovl,
                origin,
                receipt,
                data,
                points,
            )?);
        }
        let systems: [_; 2] = prepared.try_into().unwrap();
        let origins = SystemLayoutOrigins {
            system2: systems[0].origin.clone(),
            system3: systems[1].origin.clone(),
        };
        Ok(PreparedHighSystemLayouts { systems, origins })
    }

    /// Validate both resident identities before any write, then replace only
    /// their presentation tables/provenance. No layers or assets are reloaded.
    pub fn commit_high_system_layout_refresh(
        &mut self,
        stage: PreparedHighSystemLayouts,
    ) -> Result<SystemLayoutOrigins, SystemLayoutRefreshError> {
        let mut indexes = [0; 2];
        for (seat, system) in stage.systems.iter().enumerate() {
            let system_level = system.system_level;
            let stale = || SystemLayoutRefreshError::StaleStage { system_level };
            let index = self
                .layers
                .iter()
                .rposition(|layer| layer.state.system_level == Some(system_level))
                .ok_or_else(stale)?;
            let layer = &self.layers[index];
            let receipt = layer.system_layout_receipt.as_ref().ok_or_else(stale)?;
            if !std::sync::Arc::ptr_eq(&receipt.intrinsic, &system.expected_intrinsic)
                || receipt.origin != system.expected_origin
                || layer.state.fixup_data.as_ref().map(|table| &table.entries)
                    != Some(&system.expected_data)
                || layer.state.fixup_code.as_ref().map(|table| &table.entries)
                    != Some(&system.expected_points)
            {
                return Err(stale());
            }
            indexes[seat] = index;
        }
        for (index, system) in indexes.into_iter().zip(stage.systems) {
            let layer = &mut self.layers[index];
            layer.state.fixup_data = Some(system.data);
            layer.state.fixup_code = Some(system.points);
            layer.system_layout_receipt.as_mut().unwrap().origin = system.origin;
        }
        Ok(stage.origins)
    }
}

#[cfg(test)]
mod tests;
