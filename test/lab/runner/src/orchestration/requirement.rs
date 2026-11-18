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
    pub full_product_qualified: bool,
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
        derived_full_product: bool,
    ) -> Assessment {
        let full_product_qualified = derived_full_product && !failed && owned_cleanup;
        if failed {
            return Assessment {
                exit_code: 1,
                satisfied: false,
                full_product_qualified,
            };
        }
        match self {
            Self::Scoped => Assessment {
                exit_code: 0,
                satisfied: true,
                full_product_qualified,
            },
            // Exit 2 while any required product case is unavailable.
            Self::FullProduct => Assessment {
                exit_code: if full_product_qualified { 0 } else { 2 },
                satisfied: full_product_qualified,
                full_product_qualified,
            },
            Self::GoComponent => {
                let satisfied = validated_go_component && owned_cleanup;
                Assessment {
                    exit_code: if satisfied { 0 } else { 1 },
                    satisfied,
                    full_product_qualified,
                }
            }
        }
    }
    pub fn report(self, result: &Assessment) -> Value {
        json!({"schema":"humanymous.lab-requirement/1","kind":match self{Self::Scoped=>"scoped",Self::FullProduct=>"full-product",Self::GoComponent=>"go-component"},"satisfied":result.satisfied,"full_product_qualified":result.full_product_qualified})
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn bounded_success_needs_validated_eight_case_evidence_cleanup_and_no_failure() {
        for verified in [false, true] {
            for cleanup in [false, true] {
                for failed in [false, true] {
                    let outcome =
                        super::Requirement::GoComponent.assess(failed, verified, cleanup, false);
                    let valid = verified && cleanup && !failed;
                    assert_eq!(outcome.satisfied, valid);
                    assert_eq!(outcome.exit_code, if valid { 0 } else { 1 });
                    assert!(!outcome.full_product_qualified);
                    let full =
                        super::Requirement::FullProduct.assess(failed, verified, cleanup, false);
                    assert!(!full.satisfied);
                    assert_ne!(full.exit_code, 0);
                    // Only a derived complete claim, with no failure and owned
                    // cleanup, may satisfy the full-product requirement.
                    let derived =
                        super::Requirement::FullProduct.assess(failed, verified, cleanup, true);
                    assert_eq!(derived.satisfied, cleanup && !failed);
                    assert_eq!(derived.exit_code == 0, cleanup && !failed);
                    assert_eq!(derived.full_product_qualified, cleanup && !failed);
                }
            }
        }
        assert!(super::Requirement::select(true, true, true).is_err());
        assert!(super::Requirement::select(false, true, false).is_err());
        let mode = super::Requirement::select(false, true, true).unwrap();
        let report = mode.report(&mode.assess(false, true, true, false));
        assert_eq!(report["kind"], "go-component");
        assert_eq!(report["satisfied"], true);
        assert_eq!(report["full_product_qualified"], false);
    }
}
