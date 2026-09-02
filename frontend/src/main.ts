import "@fontsource-variable/manrope";
import { createPinia } from "pinia";
import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "./styles/main.css";
import { initializeTheme } from "./theme/theme";

initializeTheme();
createApp(App).use(createPinia()).use(router).mount("#app");
