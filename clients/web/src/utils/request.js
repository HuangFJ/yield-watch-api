import fetch from 'dva/fetch';
import { URL } from 'whatwg-url';
import { API_BASE_URL } from '../constants';

function parseJSON(response) {
  return response.json();
}

function checkStatus(response) {
  if (response.status >= 200 && response.status < 300) {
    return response;
  }

  const error = new Error(response.statusText);
  error.response = response;
  throw error;
}

/**
 * Requests a URL, returning a promise.
 *
 * @param  {string} url       The URL we want to request
 * @param  {object} [options] The options we want to pass to "fetch"
 * @return {object}           An object containing either "data" or "err"
 */
export default function request(url, options) {
  const { accessToken, ...opts } = options;
  let urlObj = API_BASE_URL ? new URL(url, API_BASE_URL) : new URL(url);

  if (accessToken) {
    urlObj.searchParams.set('access_token', accessToken);
  }
  return fetch(urlObj.toString(), opts)
    .then(checkStatus)
    .then(parseJSON)
    .then(data => ({ data }))
    .catch(err => ({ err }));
}
