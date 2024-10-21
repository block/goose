from sqlalchemy import Column, Integer, String
from database import Base

class Appointment(Base):
    __tablename__ = 'appointments'
    id = Column(Integer, primary_key=True)
    name = Column(String, nullable=False)
    mobile_phone = Column(String, nullable=False)
    home_address = Column(String)
    date = Column(String, nullable=False)
