import React from 'react'

export default function Box({ size } : { size: number }) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" style={{ height: size, width: size}} width={size} height={size} viewBox={`0 0 ${size} ${size}`} fill="none">
      <path d="M1 4.35L7 1.25L13 4.35M1 4.35L7 7.25M1 4.35V10.75L7 13.75M13 4.35L7 7.25M13 4.35V10.75L7 13.75M7 7.25V13.75" stroke="url(#paint0_linear_113_4015)" stroke-width="2" stroke-linecap="round"/>
      <defs>
        <linearGradient id="paint0_linear_113_4015" x1="-5" y1="-7.25" x2="27.1928" y2="20.7684" gradientUnits="userSpaceOnUse">
          <stop offset="0.230048" stop-color="#2E7CF6"/>
          <stop offset="0.430048" stop-color="#F200FF" stop-opacity="0.25"/>
          <stop offset="0.615048" stop-color="#FAC145" stop-opacity="0.669964"/>
        </linearGradient>
      </defs>
    </svg>
  )
}
