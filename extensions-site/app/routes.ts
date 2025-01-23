import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("detail/:id", "./routes/detail.tsx")
] satisfies RouteConfig;