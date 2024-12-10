import React from 'react';

export default function BottomMenu({hasMessages}) {
  return (
    <div className="flex relative text-bottom-menu dark:text-bottom-menu-dark pl-[15px] text-[10px] h-[30px] leading-[30px] align-middle bg-bottom-menu-background dark:bg-bottom-menu-background-dark rounded-b-2xl">
      <span
        className="cursor-pointer"
        onClick={async () => {
          console.log("Opening directory chooser");
          if (hasMessages) {
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
