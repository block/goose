import { IconDownload } from "./icons/download";
import { IconGoose } from "./icons/goose";
import { ThemeToggle } from "./themeToggle";
import { Button } from "./ui/button";
import { SiteURLs } from '../constants';
import { NavLink, useLocation } from "react-router";

export const Header = () => {
  const location = useLocation();
  const { hash, pathname, search } = location;

  const stableDownload = "https://github.com/block/goose/releases/download/stable/Goose.zip";
  
  // link back to the main site if the icon is clicked on the extensions homepage
  // otherwise link back to the extensions homepage
  const gooseIconLink = pathname === "/" ? SiteURLs.GOOSE_HOMEPAGE : "/";
  return (
    <div className="bg-bgApp container mx-auto border-borderSubtle py-16">
      <div className="h-full flex justify-between items-center">
        <NavLink to={gooseIconLink} className="text-textProminent">
          <IconGoose />
        </NavLink>
        <div className="w-auto items-center flex">
          <Button>
            <a href={stableDownload}>
              <IconDownload /></a>
              <span className="ml-2"><a href={stableDownload}>Download Goose for desktop</a></span>
            
          </Button>
          <ThemeToggle className="ml-4" />
        </div>
      </div>
    </div>
  );
};
