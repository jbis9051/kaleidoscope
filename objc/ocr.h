#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>

typedef struct OCRResult {
    char* text;
    float origin_x;
    float origin_y;
    float size_width;
    float size_height;
} OCRResult;
void OCRResult_cleanup(OCRResult* result, size_t count);

OCRResult* perform_ocr(char* path, size_t* count);


#ifdef __cplusplus
}
#endif