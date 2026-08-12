// src/components/ImageCarousel.js
import React from 'react';
import { Swiper, SwiperSlide } from 'swiper/react';
import { Navigation, Pagination } from 'swiper/modules';
import 'swiper/css';
import 'swiper/css/navigation';
import 'swiper/css/pagination';

const ImageCarousel = ({ images, id, width = '100%', names = [] }) => {
  const [activeIndex, setActiveIndex] = React.useState(0);

  // Get the current image name from the names array if available
  const getCurrentImageName = () => {
    if (Array.isArray(names) && names.length > activeIndex && names[activeIndex]) {
      return names[activeIndex];
    }
    
    // Don't show anything if no names provided
    return '';
  };

  return (
    <div className="carousel-container">
      {getCurrentImageName() && (
        <h3 className="carousel-header">{getCurrentImageName()}</h3>
      )}
    
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
