import { sms, auth } from '../services/api';
import {routerRedux} from 'dva/router';

export default {

  namespace: 'login',

  state: {
    interval: 0,
  },

  effects: {

    *sms({ payload }, { call, put }) {
      const data = yield call(sms, payload);
      console.log(data);
      yield put({ type: 'updateState', payload: data });
    },

    *smsAuth({ payload }, { call, put }) {
      const data = yield call(auth, payload);
      console.log(data);
      yield put(routerRedux.push({
        pathname: '/dashboard',
      }));
    },

  },

  reducers: {

    updateState(state, action) {
      return { ...state, ...action.payload };
    },

  },

};
