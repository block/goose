import React from 'react';

export default function BottomMenu({isWindowTainted}) {
  return (
    <div className="flex relative text-bottom-menu pl-[15px] text-[10px] bg-white h-[30px] leading-[30px] align-middle bg-white rounded-b-2xl">
      <span
        className="cursor-pointer"
        onClick={async () => {
          console.log("Opening directory chooser");
          if (isWindowTainted) {
            window.electron.directoryChooser();
          } else {
            window.electron.directoryChooser(true);  
          }          
      }}>
        Working in {window.appConfig.get("GOOSE_WORKING_DIR")}
      </span>
    </div>
  );
}
