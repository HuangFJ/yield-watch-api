import { my_coins, my_values } from '../services/api';

export default {

    namespace: 'dashboard',

    subscriptions: {

        setup({ dispatch, history }) {
            history.listen(({ pathname }) => {
                if (pathname === '/dashboard' || pathname === '/') {
                    dispatch({ type: 'myCoins' });
                    dispatch({ type: 'myValues' });
                }
            });
        },

    },

    state: {
        coinList: {
            balance: 0,
            states: [],
        },
        values: [],
    },

    effects: {

        *myCoins(_, { call, put, select }) {
            const coinList = yield call(my_coins);
            yield put({ type: 'updateState', payload: { coinList } });
        },

        *myValues(_, { call, put }) {
            const values = yield call(my_values);
            yield put({ type: 'updateState', payload: { values } });
        },

    },

    reducers: {

        updateState(state, { payload }) {
            return {
                ...state,
                ...payload,
            }
        },

    },

}