// src/components/ImageCarousel.js
import React from 'react';
import { Swiper, SwiperSlide } from 'swiper/react';
import { Navigation, Pagination } from 'swiper/modules';
import 'swiper/css';
import 'swiper/css/navigation';
import 'swiper/css/pagination';

const ImageCarousel = ({ images, id }) => {
  const [activeIndex, setActiveIndex] = React.useState(0);

  const getCurrentImageName = () => {
    const currentImage = images[activeIndex];
    const imagePath = typeof currentImage === 'string' ? currentImage : (currentImage?.src || '');
    
    // Extract filename from path and remove extension
    const filename = imagePath.split('/').pop() || '';
    const lastDotIndex = filename.lastIndexOf('.');
    
    return lastDotIndex !== -1 ? filename.substring(0, lastDotIndex) : filename;
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
        style={{ width: '100%' }}
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
