import React from 'react';
import { Swiper, SwiperSlide } from 'swiper/react';
import { Navigation, Pagination, FreeMode } from 'swiper/modules';
import 'swiper/css';
import 'swiper/css/navigation';
import 'swiper/css/pagination';
import 'swiper/css/free-mode';
import ContentCard from './ContentCard';

type ContentType = 'video' | 'blog' ;

interface ContentItem {
  type: ContentType;
  title: string;
  description: string;
  thumbnailUrl?: string;
  linkUrl: string;
  date?: string;
  duration?: string;
}

interface ContentCardCarouselProps {
  items: ContentItem[];
  size?: 'large' | 'compact';
  showNavigation?: boolean;
  showPagination?: boolean;
}

const carouselStyles = {
  container: {
    margin: '2rem 0',
  },
  swiperContainer: {
    paddingBottom: '2rem', // Space for pagination dots
  },
};

export default function ContentCardCarousel({
  items,
  size,
  showNavigation = true,
  showPagination = true,
}: ContentCardCarouselProps) {
  return (
    <div style={carouselStyles.container}>
      <Swiper
        slidesPerView="auto"
        spaceBetween={16}
        freeMode={false}
        navigation={showNavigation}
        pagination={showPagination ? { 
          clickable: true
        } : false}
        modules={[Navigation, Pagination, FreeMode]}
        style={carouselStyles.swiperContainer}
      >
          {items.map((item, index) => (
            <SwiperSlide key={index} style={{ 
              width: size === 'large' ? 'min(500px, 90vw)' : 'min(350px, 85vw)',
              minWidth: size === 'large' ? '300px' : '250px'
            }}>
              <ContentCard
                type={item.type}
                title={item.title}
                description={item.description}
                thumbnailUrl={item.thumbnailUrl}
                linkUrl={item.linkUrl}
                date={item.date}
                duration={item.duration}
                size={size}
              />
            </SwiperSlide>
          ))}
      </Swiper>
    </div>
  );
}
