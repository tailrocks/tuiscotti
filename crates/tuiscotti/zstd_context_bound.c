/* Include every conditional field and the maximum capped literal buffer. */
#undef DYNAMIC_BMI2
#define DYNAMIC_BMI2 1
#undef ZSTD_LEGACY_SUPPORT
#define ZSTD_LEGACY_SUPPORT 1
#undef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
#define FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION 1
#undef ZSTD_NO_TRACE
#undef ZSTD_TRACE
#define ZSTD_TRACE 1
#undef ZSTD_DECODER_INTERNAL_BUFFER
#define ZSTD_DECODER_INTERNAL_BUFFER (128 << 10)
#include "decompress/zstd_decompress_internal.h"

typedef char tuiscotti_zstd_version_must_match[(ZSTD_VERSION_NUMBER == 10507) ? 1 : -1];
typedef char tuiscotti_zstd_context_must_fit[(sizeof(struct ZSTD_DCtx_s) <= 1073741824) ? 1 : -1];
