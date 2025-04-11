import { createHashRouter } from "react-router";
import HomePage from "./routes/home";
import DetailPage from "./routes/detail";

export const router = createHashRouter([
  {
    path: "/",
    element: <HomePage />,
  },
  {
    path: "/detail/:id",
    element: <DetailPage />,
  },
]);
