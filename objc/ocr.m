#import "ocr.h"
#import <Cocoa/Cocoa.h>
#import <Vision/Vision.h>

NSMutableArray* performOCROnImageAtPath(NSString* path) {
    NSImage *image = [[NSImage alloc] initWithContentsOfFile:path];

    CGImageRef cgImage = [image CGImageForProposedRect:NULL context:NULL hints:NULL];

    if (!cgImage) {
        NSLog(@"performOCROnImageAtPath: failed to get CGImage");
        return nil;
    }

    NSMutableArray *output = [NSMutableArray array];

    VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] initWithCompletionHandler:^(VNRequest *req, NSError *error) {
        if (error) {
            NSLog(@"performOCROnImageAtPath: %@", error.localizedDescription);
            return;
        }

        for (VNRecognizedTextObservation *observation in req.results) {
            VNRecognizedText *text = [observation topCandidates:1].firstObject;
            CGRect boundingBox = observation.boundingBox;
            if (text) {
                NSDictionary *result = @{
                        @"text": text.string,
                        @"boundingBox": [NSValue valueWithRect:boundingBox],
                };
                [output addObject:result];
            }
        }
    }];

    VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithCGImage:cgImage options:@{}];
    [handler performRequests:@[request] error:nil]; // this blocks until the request is done
    return output;
}

OCRResult* perform_ocr(char* path, size_t* count) {
    NSString* imagePath = [NSString stringWithUTF8String:path];
    NSMutableArray* output = performOCROnImageAtPath(imagePath);
    if (!output) {
        NSLog(@"perform_ocr: failed to perform OCR");
        return NULL;
    }

    OCRResult* result = malloc(sizeof(OCRResult) * output.count);
    *count = (size_t)output.count;

    for (size_t i = 0; i < output.count; i++) {
        NSDictionary* dict = output[i];
        NSString* text = dict[@"text"];
        NSValue* boundingBox = dict[@"boundingBox"];
        NSRect rect = [boundingBox rectValue];
        result[i].text = strdup([text UTF8String]);
        result[i].origin_x = rect.origin.x;
        result[i].origin_y = rect.origin.y;
        result[i].size_width = rect.size.width;
        result[i].size_height = rect.size.height;
    }

    return result;
}

void OCRResult_cleanup(OCRResult* result, size_t count) {
    for (size_t i = 0; i < count; i++) {
        free(result[i].text);
    }
    free(result);
}