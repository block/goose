// src/components/ImageCarousel.js
import React from 'react';
import { Swiper, SwiperSlide } from 'swiper/react';
import { Navigation, Pagination } from 'swiper/modules';
import 'swiper/css';
import 'swiper/css/navigation';
import 'swiper/css/pagination';

const ImageCarousel = ({ images, id, width = 100% }) => {
  const [activeIndex, setActiveIndex] = React.useState(0);

  const getCurrentImageName = () => {
    const currentImage = images[activeIndex];
    // Get the source path as a string
    const imagePath = typeof currentImage === 'string' ? currentImage : (currentImage?.toString() || '');
    
    // Try to extract the actual filename from the path by finding the last part of the path that matches a model name pattern
    const matches = imagePath.match(/\/([^\/]+)\.(gif|png|jpg|jpeg)/) 
                   
    if (matches && matches[1]) {
      return matches[1];
    }
    
    // Last resort fallback
    return `Image ${activeIndex + 1}`;
  };

  return (
    <div className="carousel-container">
      <h3 className="carousel-header">{getCurrentImageName()}</h3>
    
      <Swiper
        spaceBetween={10}
        slidesPerView={1}
        navigation
        pagination={{ clickable: true }}
        modules={[Navigation, Pagination]}
        className={`swiper-container-${id}`}  // Unique class for each carousel
        style={{ width: width }}
        onSlideChange={(swiper) => setActiveIndex(swiper.activeIndex)}
      >
        {images.map((src, index) => (
          <SwiperSlide key={index}>
            <img src={src} alt={`Slide ${index + 1}`} className="carousel-image" />
          </SwiperSlide>
        ))}
      </Swiper>
    </div>
  );
};


export default ImageCarousel;
