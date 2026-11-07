# humanymous-platform

Explicit platform roles for Humanymous. The default library exposes neither role
and has no normal or build dependencies.

- The testing feature exposes contract assertions, strict JSON decoding and owned
  qualification-process helpers under testing. Tools may explicitly enable it;
  this feature grants no native provider or runtime authorization.
- On Windows, windows-object exposes borrowed-file observations and the existing
  descriptor-at-create bridge under windows_object. Observations have private
  fields and are distinct from caller-supplied wire claims. This feature exposes
  no Windows bridge on other targets.

The features are independent. Selecting both is not evidence of a qualified
native installation, complete process cleanup or trusted producer. Historical
handoff notes under docs/history retain their original scope and limitations.

Licensed under Apache-2.0; see LICENSE.
