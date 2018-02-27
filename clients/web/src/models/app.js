/* global window */
/* global document */
import queryString from 'query-string';
import { me, unauth } from '../services/api';
import { routerRedux } from 'dva/router';
import { Unauthorized, UserNotFound } from '../utils/error';

export default {

    namespace: 'app',

    state: {
        user: {},
        refererPathname: '',
        refererQuery: {},
    },

    subscriptions: {

        setupHistory({ dispatch, history }) {
            history.listen(location => {
                dispatch({
                    type: 'updateState',
                    payload: {
                        refererPathname: location.pathname,
                        refererQuery: queryString.parse(location.search),
                    },
                });
            });
        },

        setup({ dispatch }) {
            dispatch({ type: 'query' });
        },

    },

    effects: {

        * query(_, { call, put, select }) {
            const ret = yield call(me);
            const { refererPathname } = yield select(_ => _.app);
            if (ret.err) {
                if (ret.err instanceof Unauthorized) {
                    yield put(routerRedux.push({
                        pathname: '/login',
                        search: queryString.stringify({
                            from: refererPathname,
                        }),
                    }))
                } else if (ret.err instanceof UserNotFound) {
                    yield put(routerRedux.push({
                        pathname: '/register',
                    }))
                }
            } else {
                const user = ret.data;
                yield put({
                    type: 'updateState',
                    payload: { user },
                });
            }
        },

        * logout(_, { call, put }) {
            yield call(unauth);
            yield put({ type: 'query' })
        }

    },

    reducers: {

        updateState(state, { payload }) {
            return {
                ...state,
                ...payload,
            }
        },

    }

}