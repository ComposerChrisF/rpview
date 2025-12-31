# Filter Architecture - Future Design Options

## Current Architecture

The current filter system (`src/utils/filters.rs`) uses:
- Individual functions for each filter type (brightness, contrast, gamma)
- Each function performs a complete pass through all image pixels
- `apply_filters()` chains them together sequentially
- Filter settings stored as a simple struct with 3 fields

### Current Performance Characteristics
- **Multi-pass approach**: Applying 3 filters requires 3 complete iterations through the image
- **Memory overhead**: Each filter creates a new intermediate image
- **Simple to understand**: Each filter is independent and easy to reason about

## Performance Optimization (Immediate Win)

Before implementing any trait system, the biggest performance gain would come from **single-pass processing**:

### Current Approach (Multi-Pass)
```rust
// Pass 1: Apply brightness to all pixels
let img1 = apply_brightness(img, brightness);
// Pass 2: Apply contrast to all pixels
let img2 = apply_contrast(&img1, contrast);
// Pass 3: Apply gamma to all pixels
let img3 = apply_gamma(&img2, gamma);
```

### Optimized Approach (Single-Pass)
```rust
// Single pass: Apply all filters to each pixel before moving to next
pub fn apply_filters_optimized(img: &DynamicImage, brightness: f32, contrast: f32, gamma: f32) -> DynamicImage {
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let mut output = ImageBuffer::new(width, height);
    
    // Pre-calculate lookup tables (done once, not per-pixel)
    let gamma_lut = build_gamma_lut(gamma);
    let contrast_factor = calculate_contrast_factor(contrast);
    let brightness_adjustment = (brightness * 2.55) as i32;
    
    // Single iteration through all pixels
    for (x, y, pixel) in rgba_img.enumerate_pixels() {
        let mut r = pixel[0];
        let mut g = pixel[1];
        let mut b = pixel[2];
        
        // Apply all filters in sequence to this pixel
        r = (r as i32 + brightness_adjustment).clamp(0, 255) as u8;
        g = (g as i32 + brightness_adjustment).clamp(0, 255) as u8;
        b = (b as i32 + brightness_adjustment).clamp(0, 255) as u8;
        
        r = apply_contrast_to_channel(r, contrast_factor);
        g = apply_contrast_to_channel(g, contrast_factor);
        b = apply_contrast_to_channel(b, contrast_factor);
        
        r = gamma_lut[r as usize];
        g = gamma_lut[g as usize];
        b = gamma_lut[b as usize];
        
        output.put_pixel(x, y, Rgba([r, g, b, pixel[3]]));
    }
    
    DynamicImage::ImageRgba8(output)
}
```

**Performance gain**: ~3x faster for 3 filters, ~N-1x faster for N filters
**Memory gain**: No intermediate images needed
**Implementation effort**: Low - just refactor existing code

---

## Future Trait-Based Architectures

### Option 1: Single-Pass Filter Pipeline (Best Performance)

**Best for**: Simple per-pixel operations (brightness, contrast, saturation, hue, tint, etc.)

```rust
/// Pure computation trait - processes individual pixels
pub trait PixelFilter: Send {
    /// Apply filter transformation to a single pixel
    fn apply_to_pixel(&self, pixel: Rgba<u8>) -> Rgba<u8>;
    
    /// Filter identifier for debugging/logging
    fn name(&self) -> &str;
    
    /// Check if filter is effectively a no-op (can be skipped)
    fn is_identity(&self) -> bool {
        false
    }
}

/// Example implementation
struct BrightnessFilter {
    adjustment: i32, // Pre-calculated from -100..100 range
}

impl PixelFilter for BrightnessFilter {
    fn apply_to_pixel(&self, pixel: Rgba<u8>) -> Rgba<u8> {
        Rgba([
            (pixel[0] as i32 + self.adjustment).clamp(0, 255) as u8,
            (pixel[1] as i32 + self.adjustment).clamp(0, 255) as u8,
            (pixel[2] as i32 + self.adjustment).clamp(0, 255) as u8,
            pixel[3],
        ])
    }
    
    fn name(&self) -> &str { "Brightness" }
    
    fn is_identity(&self) -> bool {
        self.adjustment == 0
    }
}

/// Pipeline for efficient single-pass processing
pub struct PixelFilterPipeline {
    filters: Vec<Box<dyn PixelFilter>>,
}

impl PixelFilterPipeline {
    pub fn apply(&self, img: &DynamicImage) -> DynamicImage {
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();
        let mut output = ImageBuffer::new(width, height);
        
        // Single pass through image
        for (x, y, pixel) in rgba_img.enumerate_pixels() {
            let mut result = *pixel;
            
            // Apply all filters to this pixel
            for filter in &self.filters {
                if !filter.is_identity() {
                    result = filter.apply_to_pixel(result);
                }
            }
            
            output.put_pixel(x, y, result);
        }
        
        DynamicImage::ImageRgba8(output)
    }
}
```

**Advantages**:
- Maximum performance for pixel-based filters
- Easy to add new simple filters
- Predictable performance characteristics

**Disadvantages**:
- Can't handle filters that need surrounding pixels (blur, sharpen, edge detection)
- All filters must work on Rgba<u8> (no intermediate formats)

---

### Option 2: Composable Filter Chain (Most Flexible)

**Best for**: Complex filters that need access to multiple pixels or the entire image

```rust
/// Full-image filter trait - can do anything
pub trait ImageFilter: Send + Sync {
    /// Apply filter to entire image
    fn apply(&self, img: &DynamicImage) -> DynamicImage;
    
    /// Filter identifier
    fn name(&self) -> &str;
    
    /// Check if filter is a no-op
    fn is_no_op(&self) -> bool {
        false
    }
    
    /// Optional: Estimate processing time for progress reporting
    fn estimated_complexity(&self) -> FilterComplexity {
        FilterComplexity::Medium
    }
}

#[derive(Copy, Clone)]
pub enum FilterComplexity {
    Low,      // ~1ms for 1920x1080
    Medium,   // ~10ms for 1920x1080
    High,     // ~100ms for 1920x1080
    VeryHigh, // ~1000ms for 1920x1080
}

/// Example: Gaussian blur (needs surrounding pixels)
struct GaussianBlurFilter {
    radius: u32,
}

impl ImageFilter for GaussianBlurFilter {
    fn apply(&self, img: &DynamicImage) -> DynamicImage {
        // Complex convolution operation
        img.blur(self.radius as f32)
    }
    
    fn name(&self) -> &str { "Gaussian Blur" }
    
    fn estimated_complexity(&self) -> FilterComplexity {
        match self.radius {
            0..=2 => FilterComplexity::Medium,
            3..=5 => FilterComplexity::High,
            _ => FilterComplexity::VeryHigh,
        }
    }
}

/// Pipeline that chains arbitrary filters
pub struct FilterPipeline {
    filters: Vec<Box<dyn ImageFilter>>,
}

impl FilterPipeline {
    pub fn apply(&self, img: &DynamicImage) -> DynamicImage {
        self.filters
            .iter()
            .filter(|f| !f.is_no_op())
            .fold(img.clone(), |acc, filter| {
                eprintln!("Applying filter: {}", filter.name());
                filter.apply(&acc)
            })
    }
    
    pub fn estimated_processing_time(&self) -> FilterComplexity {
        // Could sum complexities for progress bar
        self.filters
            .iter()
            .map(|f| f.estimated_complexity())
            .max()
            .unwrap_or(FilterComplexity::Low)
    }
}
```

**Advantages**:
- Maximum flexibility - any image processing operation possible
- Can implement advanced filters (FFT, convolution, morphology, etc.)
- Easy to compose and reorder filters

**Disadvantages**:
- Multi-pass by nature (slower for simple pixel operations)
- Higher memory usage (intermediate images)
- Performance varies widely by filter type

---

### Option 3: Hybrid Approach (Recommended)

**Best for**: Real-world applications with both simple and complex filters

Combines the performance of Option 1 with the flexibility of Option 2:

```rust
/// Simple per-pixel transformations
pub trait PixelFilter: Send {
    fn apply_to_pixel(&self, pixel: Rgba<u8>) -> Rgba<u8>;
    fn name(&self) -> &str;
    fn is_identity(&self) -> bool { false }
}

/// Complex image-wide transformations
pub trait ComplexFilter: Send + Sync {
    fn apply(&self, img: &DynamicImage) -> DynamicImage;
    fn name(&self) -> &str;
    fn is_no_op(&self) -> bool { false }
}

/// Two-stage pipeline
pub struct HybridFilterPipeline {
    /// All pixel filters processed in a single pass
    pixel_filters: Vec<Box<dyn PixelFilter>>,
    
    /// Complex filters applied sequentially after pixel filters
    complex_filters: Vec<Box<dyn ComplexFilter>>,
}

impl HybridFilterPipeline {
    pub fn apply(&self, img: &DynamicImage) -> DynamicImage {
        // Stage 1: Single-pass pixel filtering
        let mut result = if !self.pixel_filters.is_empty() {
            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let mut output = ImageBuffer::new(width, height);
            
            for (x, y, pixel) in rgba_img.enumerate_pixels() {
                let mut result = *pixel;
                for filter in &self.pixel_filters {
                    if !filter.is_identity() {
                        result = filter.apply_to_pixel(result);
                    }
                }
                output.put_pixel(x, y, result);
            }
            
            DynamicImage::ImageRgba8(output)
        } else {
            img.clone()
        };
        
        // Stage 2: Sequential complex filtering
        for filter in &self.complex_filters {
            if !filter.is_no_op() {
                result = filter.apply(&result);
            }
        }
        
        result
    }
}
```

**Advantages**:
- Best performance for common case (pixel filters)
- Supports advanced filters when needed
- Clear separation of concerns
- Optimal memory usage

**Disadvantages**:
- Two trait systems to maintain
- Slight additional complexity

**Recommended filter categorization**:
- **Pixel filters**: Brightness, Contrast, Gamma, Saturation, Hue Shift, Temperature, Tint, Exposure, Shadows, Highlights
- **Complex filters**: Blur, Sharpen, Edge Detection, Noise Reduction, Unsharp Mask, Vignette

---

## GUI Integration

Separate the GUI controls from filter processing logic:

```rust
// In src/components/filter_controls.rs

/// Trait for filter control widgets
pub trait FilterControlWidget {
    /// Render the control UI (sliders, buttons, etc.)
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement;
    
    /// Get current filter parameters
    fn get_settings(&self) -> FilterSettings;
    
    /// Set filter parameters (for loading saved state)
    fn set_settings(&mut self, settings: FilterSettings);
    
    /// Check if settings have changed since last check
    fn has_changed(&mut self) -> bool;
}

/// Extensible filter settings enum
#[derive(Clone, Debug, PartialEq)]
pub enum FilterSettings {
    Brightness(f32),
    Contrast(f32),
    Gamma(f32),
    Saturation(f32),
    Blur { radius: f32 },
    Sharpen { amount: f32 },
    // Easy to add new filter types
}

/// Current approach (can keep this for now)
pub struct FilterControls {
    brightness: f32,
    contrast: f32,
    gamma: f32,
}

impl FilterControls {
    pub fn to_filter_settings(&self) -> Vec<FilterSettings> {
        vec![
            FilterSettings::Brightness(self.brightness),
            FilterSettings::Contrast(self.contrast),
            FilterSettings::Gamma(self.gamma),
        ]
    }
}
```

**Key principle**: GUI and processing logic remain decoupled through settings/config objects.

---

## Migration Path

### Phase 1: Immediate Optimization (Low effort, high impact)
1. Refactor `apply_filters()` to use single-pass processing
2. Keep existing API and `FilterSettings` struct
3. No trait system needed yet
4. **Estimated time**: 2-4 hours
5. **Performance gain**: 2-3x faster

### Phase 2: Add More Pixel Filters (Medium effort)
1. Implement Option 1 (PixelFilter trait)
2. Migrate existing filters to trait implementations
3. Add new filters: saturation, hue, temperature, tint, exposure
4. Keep GUI controls simple (sliders)
5. **Estimated time**: 1-2 days
6. **Benefit**: Easy to add new simple filters

### Phase 3: Add Complex Filters (High effort)
1. Implement Option 3 (Hybrid approach)
2. Add ComplexFilter trait for advanced operations
3. Implement: blur, sharpen, noise reduction
4. May need more sophisticated GUI controls
5. **Estimated time**: 3-5 days
6. **Benefit**: Professional-grade image editing capabilities

### Phase 4: Advanced Features (Optional)
1. Filter presets (save/load combinations)
2. Real-time preview with region selection
3. Before/after comparison view
4. Filter layer system with blending modes
5. **Estimated time**: 1-2 weeks per feature
6. **Benefit**: Approaches professional image editing software

---

## Progress Reporting for Complex Filters

For a real progress bar (0-100%), you'd need filters to report progress:

```rust
pub trait ProgressReporter: Send {
    fn report_progress(&self, current: usize, total: usize);
}

pub trait ProgressiveFilter: Send + Sync {
    fn apply_with_progress(
        &self, 
        img: &DynamicImage, 
        reporter: &dyn ProgressReporter
    ) -> DynamicImage;
    
    fn name(&self) -> &str;
}

// Example implementation
impl ProgressiveFilter for BrightnessFilter {
    fn apply_with_progress(
        &self, 
        img: &DynamicImage, 
        reporter: &dyn ProgressReporter
    ) -> DynamicImage {
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();
        let total_pixels = (width * height) as usize;
        let mut output = ImageBuffer::new(width, height);
        
        let mut processed = 0;
        let report_interval = total_pixels / 100; // Report every 1%
        
        for (x, y, pixel) in rgba_img.enumerate_pixels() {
            // Apply filter...
            output.put_pixel(x, y, transformed_pixel);
            
            processed += 1;
            if processed % report_interval == 0 {
                reporter.report_progress(processed, total_pixels);
            }
        }
        
        reporter.report_progress(total_pixels, total_pixels); // 100%
        DynamicImage::ImageRgba8(output)
    }
}
```

This would allow the GUI to show a real progress bar, but adds complexity. May not be needed for fast filters.

---

## Recommendations Summary

1. **Short term** (next session): Implement single-pass optimization
   - Quick win, no architectural changes
   - 2-3x performance improvement

2. **Medium term** (when adding more filters): Implement Hybrid approach (Option 3)
   - Separate PixelFilter and ComplexFilter traits
   - Optimal performance for both simple and complex filters
   - Room to grow

3. **Long term** (if building a full image editor): 
   - Add progress reporting for long-running filters
   - Implement filter presets and layer system
   - Consider GPU acceleration for complex filters

4. **GUI principle**: Always keep GUI and processing separated
   - Controls emit settings/parameters
   - Processing consumes settings/parameters
   - Makes testing and maintenance easier

The architecture you choose depends on your goals:
- **Just better filters for image viewing**: Stick with current + optimization
- **Adding several more filters**: Go with Hybrid approach
- **Building an image editor**: Full trait system with progress reporting
