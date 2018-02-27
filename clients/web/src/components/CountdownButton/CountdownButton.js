import React from 'react';
import { Button } from 'antd-mobile';
import PropTypes from 'prop-types';

class CountdownButton extends React.Component {
    constructor(props) {
        super(props);

        this.state = {

        }
    }
    componentDidUpdate(){
        
    }
    render() {
        return (
            <Button></Button>
        )
    }
}

CountdownButton.propTypes = {
    countdown: PropTypes.number
}

export default CountdownButton;