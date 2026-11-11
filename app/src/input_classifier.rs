use std::sync::Arc;

#[cfg(feature = "input_classification")]
use input_classifier::HeuristicClassifier;
use input_classifier::InputClassifier;
#[cfg(any(
    feature = "nld_classifier_v1",
    feature = "nld_classifier_v2",
    feature = "nld_classifier_v3"
))]
use input_classifier::{OnnxClassifier, OnnxModel};
use warpui::{Entity, ModelContext, SingletonEntity};

pub struct InputClassifierModel {
    classifier: Option<Arc<dyn InputClassifier>>,
}

impl InputClassifierModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        if crate::local_terminal::is_enabled(ctx) {
            return Self { classifier: None };
        }
        #[cfg(not(feature = "input_classification"))]
        {
            Self { classifier: None }
        }
        #[cfg(feature = "input_classification")]
        {
            #[cfg(feature = "nld_classifier_v1")]
            {
                match OnnxClassifier::new(OnnxModel::BertTinyV1) {
                    Ok(classifier) => {
                        log::info!("Loaded onnx classifier bert_tiny_v1.onnx");
                        return Self {
                            classifier: Some(Arc::new(classifier)),
                        };
                    }
                    Err(e) => log::warn!("Failed to load onnx classifier bert_tiny_v1.onnx: {e:#}"),
                }
            }

            #[cfg(feature = "nld_classifier_v2")]
            {
                match OnnxClassifier::new(OnnxModel::BertTinyV2) {
                    Ok(classifier) => {
                        log::info!("Loaded onnx classifier bert_tiny_v2.onnx");
                        return Self {
                            classifier: Some(Arc::new(classifier)),
                        };
                    }
                    Err(e) => log::warn!("Failed to load onnx classifier bert_tiny_v2.onnx: {e:#}"),
                }
            }

            #[cfg(feature = "nld_classifier_v3")]
            {
                match OnnxClassifier::new(OnnxModel::BertTinyV3) {
                    Ok(classifier) => {
                        log::info!("Loaded onnx classifier bert_tiny_v3.onnx");
                        return Self {
                            classifier: Some(Arc::new(classifier)),
                        };
                    }
                    Err(e) => log::warn!("Failed to load onnx classifier bert_tiny_v3.onnx: {e:#}"),
                }
            }

            Self {
                classifier: Some(Arc::new(HeuristicClassifier)),
            }
        }
    }

    pub fn classifier(&self) -> Option<Arc<dyn InputClassifier>> {
        self.classifier.clone()
    }
}

impl Entity for InputClassifierModel {
    type Event = ();
}

impl SingletonEntity for InputClassifierModel {}

#[cfg(test)]
#[path = "input_classifier_tests.rs"]
mod tests;
