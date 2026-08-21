// Verify-only ML-DSA-65 runtime configuration for Fcitx5 for Windows Next.
//
// This product runtime must not ship private signing material. The linked
// mldsa-native object therefore exposes only the public verification API for
// the ML-DSA-65 parameter set.
#pragma once

#define MLD_CONFIG_PARAMETER_SET 65
#define MLD_CONFIG_NAMESPACE_PREFIX fcitx5_mldsa65
#define MLD_CONFIG_NO_KEYPAIR_API
#define MLD_CONFIG_NO_SIGN_API
#define MLD_CONFIG_NO_RANDOMIZED_API
#define MLD_CONFIG_NO_ASM
