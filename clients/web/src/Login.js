import React, { Component } from 'react';
import { List, InputItem } from 'antd-mobile';
import { createForm } from 'rc-form';
import Service from './api/service';

class Login extends Component {
    constructor(props) {
        super(props);
        this.state = {
            mobile: ''
        }
    }


    sms = (e) => {
        this.props.form.validateFields((error, value) => {
            console.log(error, value);
          });

        e.preventDefault();
        const { mobile } = this.state;

        Service
            .sms(mobile)
            .then((response) => {
                console.log(response);
            });
    }

    render() {
        const { getFieldProps,formItemLayout } = this.props.form;
        const {mobile} = this.state;
        return (
            <div>
                <List>
                    <InputItem 
                    type="phone" 
                    {...getFieldProps('mobile')} 
                    {...formItemLayout} 
                    label="手机号" >
                    clear 
                    placeholder="手机号码">
                        
                    </InputItem>
                    <List.Item>
                        <div style={{ width:'100%', color:'#108ee9', textAlign:'center'}} onClick={this.sms}>
                            获取验证码
                        </div>
                    </List.Item>
                </List>
            </div>
        );
    }
}

const LoginForm = createForm()(Login);
export default LoginForm;


