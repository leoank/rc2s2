// GPU load generator for EXP-0009: a Metal compute kernel dispatched back to back
// for N seconds (default 8). No Xcode needed; built in the spike dev shell with
//   clang -fobjc-arc -framework Metal -framework Foundation gpu_load.m -o gpu_load
#import <Metal/Metal.h>
#import <Foundation/Foundation.h>

int main(int argc, char **argv) {
  @autoreleasepool {
    double secs = argc > 1 ? atof(argv[1]) : 8;
    id<MTLDevice> dev = MTLCreateSystemDefaultDevice();
    NSError *err = nil;
    NSString *src = @"#include <metal_stdlib>\nusing namespace metal;\n"
      "kernel void burn(device float *out [[buffer(0)]], uint id [[thread_position_in_grid]]) {"
      "  float x = float(id) * 1e-6f;"
      "  for (int i = 0; i < 20000; i++) { x = sin(x) * cos(x) + 0.5f; }"
      "  out[id] = x; }";
    id<MTLLibrary> lib = [dev newLibraryWithSource:src options:nil error:&err];
    if (!lib) { NSLog(@"%@", err); return 1; }
    id<MTLComputePipelineState> pso =
      [dev newComputePipelineStateWithFunction:[lib newFunctionWithName:@"burn"] error:&err];
    if (!pso) { NSLog(@"%@", err); return 1; }
    id<MTLCommandQueue> q = [dev newCommandQueue];
    NSUInteger n = 1 << 22;
    id<MTLBuffer> buf = [dev newBufferWithLength:n * 4 options:MTLResourceStorageModeShared];
    NSDate *end = [NSDate dateWithTimeIntervalSinceNow:secs];
    int iters = 0;
    while ([end timeIntervalSinceNow] > 0) {
      id<MTLCommandBuffer> cb = [q commandBuffer];
      id<MTLComputeCommandEncoder> enc = [cb computeCommandEncoder];
      [enc setComputePipelineState:pso];
      [enc setBuffer:buf offset:0 atIndex:0];
      NSUInteger w = pso.maxTotalThreadsPerThreadgroup;
      [enc dispatchThreads:MTLSizeMake(n, 1, 1) threadsPerThreadgroup:MTLSizeMake(w, 1, 1)];
      [enc endEncoding];
      [cb commit];
      [cb waitUntilCompleted];
      iters++;
    }
    printf("gpu load done: %d dispatches in %.0fs\n", iters, secs);
  }
  return 0;
}
