from sqlalchemy.orm import Session
from models import Appointment

def update_user_info(db_session: Session, user_id: int, name: str, mobile_phone: str, home_address: str):
    user = db_session.query(Appointment).filter(Appointment.id == user_id).first()
    if user:
        user.name = name
        user.mobile_phone = mobile_phone
        user.home_address = home_address
        db_session.commit()
        return True
    return False