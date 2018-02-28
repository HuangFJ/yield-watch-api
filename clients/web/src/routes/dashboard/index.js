import React from 'react';
import { connect } from 'dva';
import PropTypes from 'prop-types';
import styles from './index.less';
import {CoinList} from './components';

const Dashboard = ({ dashboard, loading }) => {
    return (
        <div>
            <CoinList data={dashboard.coinList.states} />
        </div>
    )
}

Dashboard.propTypes = {
    dashboard: PropTypes.object,
    loading: PropTypes.object,
}

export default connect(({ dashboard, loading }) => ({dashboard, loading}))(Dashboard);