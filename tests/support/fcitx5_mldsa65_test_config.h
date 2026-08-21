// Unit-test-only ML-DSA-65 configuration.
//
// This target exposes deterministic keypair/sign internals only to generate
// reproducible in-memory fixtures for verifier tests. Product targets link the
// verify-only configuration in src/package/fcitx5_mldsa65_config.h instead.
#pragma once

#define MLD_CONFIG_PARAMETER_SET 65
#define MLD_CONFIG_NAMESPACE_PREFIX fcitx5_mldsa65_test
#define MLD_CONFIG_NO_RANDOMIZED_API
#define MLD_CONFIG_NO_ASM
