# MBF Site
This is the project for the MBF frontend/website.

The backend needs to be built and placed at `./public/mbf-agent/mbf-agent` or the app will not work!

Vite is used for development/bundling.

In the project directory, you can run:
### `yarn start`

Runs the app in the development mode.\
Open [http://localhost:3000](http://localhost:3000) to view it in the browser.

The page will reload if you make edits.\
NB: If testing on a non-localhost device, you must set `HOST=0.0.0.0` and `HTTPS=true`. \
HTTPS is necessary as WebUSB is only allowed in secure contexts!

### `yarn build`

Builds the app for production to the `build` folder.\
It correctly bundles React in production mode and optimizes the build for the best performance.

Your app is ready to be deployed!