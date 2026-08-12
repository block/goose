import React from 'react';
import Admonition from '@theme/Admonition';

const YouTubeShortEmbed = ({ videoUrl }) => (
  <div>
  <Admonition type="info" icon="🎥" title="Plug & Play" className='alert--video'>
    <details>
      <summary>Watch the demo</summary>
      <div style={{ textAlign: 'center', margin: '20px 0' }}>
          <iframe
          width="100%"
          height="540"
          src={videoUrl}
          title="YouTube Short"
          frameBorder="0"
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
          allowFullScreen
          ></iframe>
      </div>
    </details>
  </Admonition>
  <hr></hr>
</div>
);

export default YouTubeShortEmbed;