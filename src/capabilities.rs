//! Native browser capability expectations for desktop Chrome profiles.

/// A browser capability that CreepJS can observe as a platform signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NativeCapability {
    /// The native Web Share API (`navigator.share` and `navigator.canShare`).
    WebShare,
    /// The native Contacts Manager API.
    ContactsManager,
    /// The native Content Index API.
    ContentIndex,
    /// The native `NetworkInformation.prototype.downlinkMax` property.
    NetworkInformationDownlinkMax,
}

/// How a native Chrome runtime is expected to expose a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapabilityExpectation {
    /// The capability is expected on the target native desktop Chrome.
    Expected,
    /// The capability is not expected on the target desktop platform.
    NotExpected,
    /// The capability depends on the concrete browser build or runtime.
    RuntimeDependent,
}

impl CapabilityExpectation {
    pub(crate) const fn matches(self, observed: bool) -> bool {
        match self {
            Self::Expected => observed,
            Self::NotExpected => !observed,
            Self::RuntimeDependent => true,
        }
    }
}

/// Platform-aware expectations for native browser capabilities.
///
/// These values describe what should be validated against a real browser. They
/// do not cause Chromium to expose or hide an API, and they should not be used
/// to justify JavaScript shims for APIs that require operating-system support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCapabilityExpectations {
    /// Expected Web Share availability.
    pub web_share: CapabilityExpectation,
    /// Expected Contacts Manager availability.
    pub contacts_manager: CapabilityExpectation,
    /// Expected Content Index availability.
    pub content_index: CapabilityExpectation,
    /// Expected `NetworkInformation.prototype.downlinkMax` availability.
    pub network_information_downlink_max: CapabilityExpectation,
}

impl NativeCapabilityExpectations {
    /// Returns the expectation for one capability.
    pub const fn for_capability(self, capability: NativeCapability) -> CapabilityExpectation {
        match capability {
            NativeCapability::WebShare => self.web_share,
            NativeCapability::ContactsManager => self.contacts_manager,
            NativeCapability::ContentIndex => self.content_index,
            NativeCapability::NetworkInformationDownlinkMax => {
                self.network_information_downlink_max
            }
        }
    }

    /// Compares native page observations with the expected platform behavior.
    pub fn mismatches(
        self,
        observed: NativeCapabilityObservation,
    ) -> Vec<NativeCapabilityMismatch> {
        let observations = [
            (
                NativeCapability::WebShare,
                self.web_share,
                observed.web_share,
            ),
            (
                NativeCapability::ContactsManager,
                self.contacts_manager,
                observed.contacts_manager,
            ),
            (
                NativeCapability::ContentIndex,
                self.content_index,
                observed.content_index,
            ),
            (
                NativeCapability::NetworkInformationDownlinkMax,
                self.network_information_downlink_max,
                observed.network_information_downlink_max,
            ),
        ];

        observations
            .into_iter()
            .filter_map(|(capability, expectation, observed)| {
                (!expectation.matches(observed)).then_some(NativeCapabilityMismatch {
                    capability,
                    expectation,
                    observed,
                })
            })
            .collect()
    }
}

/// Native capability values observed from a browser page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCapabilityObservation {
    /// Whether `navigator.share` and `navigator.canShare` are available.
    pub web_share: bool,
    /// Whether the Contacts Manager API is available.
    pub contacts_manager: bool,
    /// Whether the Content Index API is available.
    pub content_index: bool,
    /// Whether `NetworkInformation.prototype.downlinkMax` is available.
    pub network_information_downlink_max: bool,
}

impl NativeCapabilityObservation {
    /// Creates an observation from the four capability-presence checks.
    pub const fn new(
        web_share: bool,
        contacts_manager: bool,
        content_index: bool,
        network_information_downlink_max: bool,
    ) -> Self {
        Self {
            web_share,
            contacts_manager,
            content_index,
            network_information_downlink_max,
        }
    }
}

/// A capability whose observed value disagrees with the selected platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCapabilityMismatch {
    /// Capability with the disagreement.
    pub capability: NativeCapability,
    /// Platform expectation used for the comparison.
    pub expectation: CapabilityExpectation,
    /// Value observed in the browser runtime.
    pub observed: bool,
}
