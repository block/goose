import { createBrowserRouter } from "react-router";
import HomePage from "./routes/home";
import DetailPage from "./routes/detail";

const basename = "/goose/v1/extensions";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <HomePage />,
  },
  {
    path: "/detail/:id",
    element: <DetailPage />,
  },
], 
{
  basename,
});
