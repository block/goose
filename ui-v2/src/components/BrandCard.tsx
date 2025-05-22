import React from 'react';

interface BrandCardProps {
  date?: Date;
  className?: string;
}

// Array of congratulatory messages for past days
const pastDayMessages = [
  { title: "Great work!", message: "You accomplished so much" },
  { title: "Well done!", message: "Another successful day" },
  { title: "Fantastic job!", message: "Making progress every day" },
  { title: "Nice one!", message: "Another day in the books" },
  { title: "Awesome work!", message: "Keep up the momentum" }
];

export default function BrandCard({ date, className = '' }: BrandCardProps) {
  const isToday = date ? new Date().toDateString() === date.toDateString() : true;

  // Get a consistent message for each date
  const getPastDayMessage = (date: Date) => {
    // Use the date's day as an index to select a message
    const index = date.getDate() % pastDayMessages.length;
    return pastDayMessages[index];
  };

  // Get message for past days
  const pastMessage = date ? getPastDayMessage(date) : pastDayMessages[0];

  return (
    <div 
      className={`
        flex flex-col justify-between 
        p-4 
        w-[366px] h-[256px] 
        ${isToday 
          ? 'bg-textStandard dark:bg-white' 
          : 'bg-gray-400/40 dark:bg-gray-400/40'
        }
        rounded-[18px]
        relative
        overflow-hidden
        transition-all duration-200
        shadow-[0_0_13.7px_rgba(0,0,0,0.04)]
        dark:shadow-[0_0_24px_rgba(255,255,255,0.02)]
        ${className}
      `}
    >
      {/* Content */}
      <div className="relative z-10 w-full">
        {/* Logo */}
        <div className={`
          w-6 h-6 
          ${isToday 
            ? '[&_path]:fill-current text-white dark:text-gray-900'
            : '[&_path]:fill-current text-white/60 dark:text-white/60'
          }
        `}>
          <svg width="24" height="23" viewBox="0 0 24 23" xmlns="http://www.w3.org/2000/svg">
            <g>
              <path d="M0.5 10.5733C0.5 8.19815 2.41385 6.27271 4.77471 6.27271H6.67984C9.04069 6.27271 10.9545 8.19816 10.9545 10.5733V18.6994C10.9545 21.0745 9.04069 23 6.67983 23H4.77471C2.41385 23 0.5 21.0745 0.5 18.6994V10.5733Z" />
              <path d="M6.67977 22.6416V23H4.77477V22.6416H6.67977ZM10.5983 18.6993V10.5733C10.5983 8.3961 8.84392 6.63109 6.67977 6.63109H4.77477C2.61062 6.63109 0.856231 8.3961 0.856231 10.5733V18.6993C0.856231 20.8766 2.61062 22.6416 4.77477 22.6416V23L4.66449 22.9986C2.39119 22.9407 0.558919 21.0974 0.501392 18.8103L0.5 18.6993V10.5733C0.5 8.23526 2.35457 6.33295 4.66449 6.27411L4.77477 6.27271H6.67977L6.7904 6.27411C9.10023 6.33306 10.9545 8.23533 10.9545 10.5733V18.6993L10.9532 18.8103C10.8956 21.0973 9.06361 22.9406 6.7904 22.9986L6.67977 23V22.6416C8.84392 22.6416 10.5983 20.8766 10.5983 18.6993Z" />
              <path d="M13.0453 4.27471C13.0453 1.91385 14.9592 0 17.3201 0H19.2252C21.586 0 23.4999 1.91385 23.4999 4.27471V6.17984C23.4999 8.54069 21.586 10.4545 19.2252 10.4545H17.3201C14.9592 10.4545 13.0453 8.54069 13.0453 6.17983V4.27471Z" />
              <path d="M19.2251 10.0983V10.4545H17.3201V10.0983H19.2251ZM23.1437 6.17977V4.27477C23.1437 2.11062 21.3893 0.356231 19.2251 0.356231H17.3201C15.156 0.356231 13.4016 2.11062 13.4016 4.27477V6.17977C13.4016 8.34392 15.156 10.0983 17.3201 10.0983V10.4545L17.2098 10.4532C14.9366 10.3956 13.1044 8.56358 13.0467 6.2904L13.0453 6.17977V4.27477C13.0453 1.95075 14.8999 0.0598847 17.2098 0.00139153L17.3201 0H19.2251L19.3357 0.00139153C21.6456 0.0599881 23.4999 1.95082 23.4999 4.27477V6.17977L23.4985 6.2904C23.4408 8.56351 21.6089 10.3955 19.3357 10.4532L19.2251 10.4545V10.0983C21.3893 10.0983 23.1437 8.34392 23.1437 6.17977Z" />
              <path d="M19.3182 14.6363C19.3182 13.4815 20.2543 12.5454 21.4091 12.5454V12.5454C22.5639 12.5454 23.5 13.4815 23.5 14.6363V14.6363C23.5 15.7911 22.5639 16.7272 21.4091 16.7272V16.7272C20.2543 16.7272 19.3182 15.7911 19.3182 14.6363V14.6363Z" />
              <path d="M23.1522 14.6361C23.1521 13.6736 22.3715 12.8932 21.4089 12.8932C20.4464 12.8933 19.6661 13.6736 19.666 14.6361C19.666 15.5988 20.4464 16.3793 21.4089 16.3794V16.7272L21.3016 16.7245C20.2324 16.6704 19.3751 15.813 19.3209 14.7438L19.3182 14.6361C19.3183 13.4815 20.2543 12.5455 21.4089 12.5454L21.5166 12.5481C22.6213 12.6041 23.5 13.5175 23.5 14.6361L23.4973 14.7438C23.4413 15.8486 22.5276 16.7272 21.4089 16.7272V16.3794C22.3716 16.3794 23.1522 15.5988 23.1522 14.6361Z" />
              <path d="M13.0453 14.6363C13.0453 13.4815 13.9815 12.5454 15.1363 12.5454V12.5454C16.291 12.5454 17.2272 13.4815 17.2272 14.6363V14.6363C17.2272 15.7911 16.291 16.7272 15.1363 16.7272V16.7272C13.9815 16.7272 13.0453 15.7911 13.0453 14.6363V14.6363Z" />
              <path d="M16.8793 14.6361C16.8793 13.6736 16.0987 12.8932 15.1361 12.8932C14.1735 12.8933 13.3932 13.6736 13.3932 14.6361C13.3932 15.5988 14.1735 16.3793 15.1361 16.3794V16.7272L15.0287 16.7245C13.9596 16.6704 13.1023 15.813 13.0481 14.7438L13.0453 14.6361C13.0454 13.4815 13.9814 12.5455 15.1361 12.5454L15.2438 12.5481C16.3485 12.6041 17.2271 13.5175 17.2272 14.6361L17.2244 14.7438C17.1685 15.8486 16.2548 16.7272 15.1361 16.7272V16.3794C16.0987 16.3794 16.8793 15.5988 16.8793 14.6361Z" />
              <path d="M0.5 2.09091C0.5 0.936132 1.45869 0 2.64129 0H8.81325C9.99586 0 10.9545 0.936132 10.9545 2.09091V2.09091C10.9545 3.24569 9.99586 4.18182 8.81325 4.18182H2.64129C1.45869 4.18182 0.5 3.24569 0.5 2.09091V2.09091Z" />
              <path d="M8.81333 3.83398V4.18182H2.64121V3.83398H8.81333ZM10.5983 2.09074C10.5983 1.12815 9.79917 0.347834 8.81333 0.347834H2.64121C1.65542 0.347891 0.85629 1.12818 0.856231 2.09074C0.856231 3.05334 1.65538 3.83393 2.64121 3.83398V4.18182L2.53128 4.1791C1.43629 4.12498 0.558276 3.26758 0.502783 2.19842L0.5 2.09074C0.500057 0.972081 1.39984 0.0586407 2.53128 0.00271745L2.64121 0H8.81333L8.92361 0.00271745C10.055 0.0587386 10.9545 0.972147 10.9545 2.09074L10.9518 2.19842C10.8963 3.26751 10.0185 4.12489 8.92361 4.1791L8.81333 4.18182V3.83398C9.79921 3.83398 10.5983 3.05338 10.5983 2.09074Z" />
              <path d="M13.1735 19.8579C13.5338 19.0233 14.546 18.6107 15.4505 18.9214L15.4934 18.9368L16.8286 19.4315L16.9187 19.4638C17.854 19.786 18.8876 19.7689 19.8114 19.4136L21.0304 18.9449L21.0731 18.9291C21.9732 18.6074 22.9911 19.0079 23.3631 19.838C23.735 20.6682 23.3225 21.6185 22.4412 21.9819L22.3989 21.9989L21.1798 22.4675C19.3933 23.1545 17.3909 23.1771 15.5885 22.5328L15.5029 22.5016L14.1678 22.0069L14.1252 21.9906C13.2389 21.6378 12.8131 20.6924 13.1735 19.8579Z" />
            </g>
          </svg>
        </div>
      </div>

      {/* Text content - bottom */}
      <div className="relative z-10 w-full flex flex-col">
        {isToday ? (
          <>
            {/* Today's content */}
            <h2 
              className={`
                font-['Cash_Sans'] font-semibold text-base
                text-white dark:text-gray-600
                tracking-[0.08em] max-w-[565px]
                mb-2
                transition-colors
              `}
            >
              Good morning
            </h2>

            <h1 
              style={{ fontWeight: 200 }}
              className={`
                font-['Cash_Sans'] text-[32px]
                text-white dark:text-gray-600
                leading-tight max-w-[565px]
                tracking-normal
                transition-colors
              `}
            >
              You've got 3 major updates this morning
            </h1>
          </>
        ) : (
          <>
            {/* Past/Future date content */}
            <h2 
              className={`
                font-['Cash_Sans'] font-semibold text-base
                text-white/60 dark:text-white/60
                tracking-[0.08em] max-w-[565px]
                mb-2
                transition-colors
              `}
            >
              {pastMessage?.title || 'Hello'}
            </h2>

            <h1 
              style={{ fontWeight: 200 }}
              className={`
                font-['Cash_Sans'] text-[32px]
                text-white/60 dark:text-white/60
                leading-tight max-w-[565px]
                tracking-normal
                transition-colors
              `}
            >
              {pastMessage?.message || 'Great work'}
            </h1>
          </>
        )}
      </div>
    </div>
  );
}
