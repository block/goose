// src/components/VideoCarousel.js
import React from 'react';
import { Swiper, SwiperSlide } from 'swiper/react';
import { Navigation, Pagination, FreeMode } from 'swiper/modules';
import 'swiper/css';
import 'swiper/css/navigation';
import 'swiper/css/pagination';
import 'swiper/css/free-mode';

const VideoCarousel = ({ videos, id, width = '100%', names = [] }) => {
  const [activeIndex, setActiveIndex] = React.useState(0);

  const getCurrentVideoName = () => {
    if (Array.isArray(names) && names.length > activeIndex && names[activeIndex]) {
      return names[activeIndex];
    }
    return '';
  };

  return (
    <div className="carousel-container">
      {getCurrentVideoName() && (
        <h3 className="carousel-header">{getCurrentVideoName()}</h3>
      )}
      <Swiper
        spaceBetween={16}
        slidesPerView={1.2}
        freeMode={true}
        navigation
        pagination={{ clickable: true }}
        modules={[Navigation, Pagination, FreeMode]}
        className={`swiper-container-${id}`}
        style={{ width: width }}
        onSlideChange={(swiper) => setActiveIndex(swiper.activeIndex)}
      >
        {videos.map((video, index) => (
          <SwiperSlide key={index}>
            <div>
              <div
                className="video-responsive video-container"
                style={{
                  position: "relative",
                  width: "100%",
                  paddingBottom: "56.25%", // 16:9 aspect ratio
                  height: 0,
                  overflow: "hidden"
                }}
              >
                {video.type === 'iframe' ? (
                  <iframe
                    src={video.src}
                    title={video.title || `Video ${index + 1}`}
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      height: "100%",
                      border: 0
                    }}
                    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                    allowFullScreen
                  ></iframe>
                ) : (
                  <video
                    controls
                    src={video.src}
                    title={video.title || `Video ${index + 1}`}
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      height: "100%",
                      border: 0
                    }}
                  />
                )}
              </div>
              {(video.description || video.duration) && (
                  <div style={{ marginTop: -32, marginBottom: 48, marginLeft: 8, fontSize: '1em', color: '#444' }}>
                  {video.description && <span>{video.description}</span>}
                  {video.duration && (
                    <span style={{ marginLeft: 8, color: '#888', fontSize: '0.95em' }}>
                        <span style={{ marginRight: 8 }}>•</span>
                        {video.duration}
                    </span>
                  )}
                </div>
              )}
            </div>
          </SwiperSlide>
        ))}
      </Swiper>
    </div>
  );
};

export default VideoCarousel;
