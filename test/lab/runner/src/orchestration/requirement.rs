//! Requested result ownership; a bounded Go pass never satisfies full product.
use crate::docker::Result;
use serde_json::{Value, json};
#[derive(Clone, Copy)]
pub(super) enum Requirement {
    Scoped,
    FullProduct,
    GoComponent,
}
pub(super) struct Assessment {
    pub exit_code: i32,
    pub satisfied: bool,
}
impl Requirement {
    pub fn select(full: bool, go: bool, go_profile: bool) -> Result<Self> {
        if full && go {
            return Err("full-product and bounded Go requirements are mutually exclusive".into());
        }
        if go && !go_profile {
            return Err("bounded Go requirement requires the dedicated Go product profile".into());
        }
        Ok(if full {
            Self::FullProduct
        } else if go {
            Self::GoComponent
        } else {
            Self::Scoped
        })
    }
    pub fn assess(
        self,
        failed: bool,
        validated_go_component: bool,
        owned_cleanup: bool,
    ) -> Assessment {
        if failed {
            return Assessment {
                exit_code: 1,
                satisfied: false,
            };
        }
        match self {
            Self::Scoped => Assessment {
                exit_code: 0,
                satisfied: true,
            },
            Self::FullProduct => Assessment {
                exit_code: 2,
                satisfied: false,
            },
            Self::GoComponent => {
                let satisfied = validated_go_component && owned_cleanup;
                Assessment {
                    exit_code: if satisfied { 0 } else { 1 },
                    satisfied,
                }
            }
        }
    }
    pub fn report(self, result: &Assessment) -> Value {
        json!({"schema":"humanymous.lab-requirement/1","kind":match self{Self::Scoped=>"scoped",Self::FullProduct=>"full-product",Self::GoComponent=>"go-component"},"satisfied":result.satisfied,"full_product_qualified":false})
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn bounded_success_needs_validated_eight_case_evidence_cleanup_and_no_failure() {
        for verified in [false, true] {
            for cleanup in [false, true] {
                for failed in [false, true] {
                    let outcome = super::Requirement::GoComponent.assess(failed, verified, cleanup);
                    let valid = verified && cleanup && !failed;
                    assert_eq!(outcome.satisfied, valid);
                    assert_eq!(outcome.exit_code, if valid { 0 } else { 1 });
                    let full = super::Requirement::FullProduct.assess(failed, verified, cleanup);
                    assert!(!full.satisfied);
                    assert_ne!(full.exit_code, 0);
                }
            }
        }
        assert!(super::Requirement::select(true, true, true).is_err());
        assert!(super::Requirement::select(false, true, false).is_err());
        let mode = super::Requirement::select(false, true, true).unwrap();
        let report = mode.report(&mode.assess(false, true, true));
        assert_eq!(report["kind"], "go-component");
        assert_eq!(report["satisfied"], true);
        assert_eq!(report["full_product_qualified"], false);
    }
}
